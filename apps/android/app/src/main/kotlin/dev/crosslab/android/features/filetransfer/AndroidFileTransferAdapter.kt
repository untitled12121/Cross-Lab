package dev.crosslab.android.features.filetransfer

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import android.util.AtomicFile
import java.io.File
import java.io.FileInputStream
import java.io.InputStream
import java.io.RandomAccessFile
import kotlin.math.min
import uniffi.crosslab_mobile_ffi.MobileFileTransferHasher
import uniffi.crosslab_mobile_ffi.MobileFileTransferOffer
import uniffi.crosslab_mobile_ffi.MobileFileTransferRecoveryKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferState
import uniffi.crosslab_mobile_ffi.MobileFileTransferVerifier

private const val IO_CHUNK_BYTES = 64 * 1024
private const val CHECKPOINT_BYTES = 1_048_576L
private const val PARTIAL_RETENTION_SECS = 7L * 24 * 60 * 60
private const val COMPLETION_RETENTION_SECS = 30L * 24 * 60 * 60

class AndroidFileTransferAdapter(
    context: Context,
) {
    private val resolver: ContentResolver = context.contentResolver
    private val root =
        context.noBackupFilesDir
            .resolve("crosslab/file-transfer")
            .apply { mkdirs() }
    private val stateFile = AtomicFile(root.resolve("state-v1.bin"))
    private val state =
        MobileFileTransferState(
            encoded =
                if (stateFile.baseFile.isFile) {
                    stateFile.readFully()
                } else {
                    null
                },
        )

    init {
        cleanupExpired()
    }

    fun persistUriPermission(
        uri: Uri,
        read: Boolean,
        write: Boolean,
    ) {
        var flags = 0
        if (read) flags = flags or Intent.FLAG_GRANT_READ_URI_PERMISSION
        if (write) flags = flags or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        require(flags != 0) { "at least one URI permission must be requested" }
        resolver.takePersistableUriPermission(uri, flags)
    }

    fun prepareSource(
        uri: Uri,
        retryOffer: MobileFileTransferOffer? = null,
    ): AndroidFileTransferSource {
        val displayName = displayName(uri)
        val hasher = MobileFileTransferHasher()
        resolver.openInputStream(uri)?.use { input ->
            val buffer = ByteArray(IO_CHUNK_BYTES)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                if (read == 0) continue
                hasher.update(buffer.copyOf(read))
            }
        } ?: throw FileTransferStorageUnavailable("selected source cannot be opened")

        val offer =
            if (retryOffer == null) {
                hasher.finishNew(displayName)
            } else {
                hasher.finishExisting(retryOffer.transferId(), displayName)
            }
        if (retryOffer != null && !offer.sameIdentity(retryOffer)) {
            throw FileTransferSourceChanged()
        }
        return AndroidFileTransferSource(uri, offer)
    }

    fun openSource(
        source: AndroidFileTransferSource,
        resumeOffset: ULong,
    ): AndroidFileTransferSourceReader {
        val fileSize = source.offer.fileSize()
        require(resumeOffset <= fileSize) { "resume offset exceeds source size" }
        val input =
            resolver.openInputStream(source.uri)
                ?: throw FileTransferStorageUnavailable("selected source cannot be reopened")
        try {
            skipExactly(input, resumeOffset.toSafeLong())
        } catch (error: Throwable) {
            input.close()
            throw error
        }
        return AndroidFileTransferSourceReader(
            input = input,
            remaining = (fileSize - resumeOffset).toSafeLong(),
        )
    }

    @Synchronized
    fun prepareReceive(
        sourceDeviceId: ByteArray,
        offer: MobileFileTransferOffer,
        destinationUri: Uri?,
        persistWritePermission: Boolean,
    ): AndroidReceivePreparation {
        val recovery = state.find(sourceDeviceId, offer)
        if (recovery.kind() == MobileFileTransferRecoveryKind.ALREADY_COMPLETE) {
            return AndroidReceivePreparation.alreadyComplete(offer.fileSize())
        }

        val durableOffset: ULong
        val resolvedUri: Uri
        when (recovery.kind()) {
            MobileFileTransferRecoveryKind.NONE -> {
                resolvedUri =
                    destinationUri
                        ?: throw FileTransferStorageUnavailable(
                            "owner destination selection is required",
                        )
                if (persistWritePermission) {
                    persistUriPermission(resolvedUri, read = false, write = true)
                }
                durableOffset = 0uL
                state.upsertPartial(
                    sourceDeviceIdBytes = sourceDeviceId,
                    offer = offer,
                    durableOffset = durableOffset,
                    locator = resolvedUri.toString().toByteArray(Charsets.UTF_8),
                    updatedAtUnixSecs = unixNow(),
                )
                commitState()
            }

            MobileFileTransferRecoveryKind.PARTIAL -> {
                durableOffset =
                    recovery.durableOffset()
                        ?: throw FileTransferStorageUnavailable("retained offset is missing")
                val retained =
                    recovery.locator()
                        ?: throw FileTransferStorageUnavailable("retained destination is missing")
                resolvedUri = Uri.parse(String(retained, Charsets.UTF_8))
                if (destinationUri != null && destinationUri != resolvedUri) {
                    throw FileTransferStorageUnavailable(
                        "retry destination does not match retained transfer state",
                    )
                }
            }

            MobileFileTransferRecoveryKind.ALREADY_COMPLETE ->
                error("already-complete state handled above")
        }

        val partial = partialFile(offer.transferId())
        recoverPartial(partial, durableOffset.toSafeLong())
        val receiver =
            AndroidFileTransferReceiver(
                adapter = this,
                sourceDeviceId = sourceDeviceId.copyOf(),
                offer = offer,
                destinationUri = resolvedUri,
                partial = partial,
                initialOffset = durableOffset.toSafeLong(),
            )
        return AndroidReceivePreparation.ready(durableOffset, receiver)
    }

    @Synchronized
    internal fun checkpoint(
        sourceDeviceId: ByteArray,
        offer: MobileFileTransferOffer,
        destinationUri: Uri,
        durableOffset: Long,
    ) {
        state.upsertPartial(
            sourceDeviceIdBytes = sourceDeviceId,
            offer = offer,
            durableOffset = durableOffset.toULong(),
            locator = destinationUri.toString().toByteArray(Charsets.UTF_8),
            updatedAtUnixSecs = unixNow(),
        )
        commitState()
    }

    @Synchronized
    internal fun completed(
        sourceDeviceId: ByteArray,
        offer: MobileFileTransferOffer,
    ) {
        state.markCompleted(
            sourceDeviceIdBytes = sourceDeviceId,
            offer = offer,
            updatedAtUnixSecs = unixNow(),
        )
        commitState()
    }

    @Synchronized
    internal fun invalidate(offer: MobileFileTransferOffer) {
        state.remove(offer.transferId())
        commitState()
    }

    internal fun publish(
        partial: File,
        destinationUri: Uri,
    ) {
        val output =
            resolver.openOutputStream(destinationUri, "wt")
                ?: throw FileTransferStorageUnavailable("destination cannot be opened")
        output.use { destination ->
            FileInputStream(partial).use { input ->
                val buffer = ByteArray(IO_CHUNK_BYTES)
                while (true) {
                    val read = input.read(buffer)
                    if (read < 0) break
                    if (read == 0) continue
                    destination.write(buffer, 0, read)
                }
                destination.flush()
            }
        }
    }

    private fun cleanupExpired() {
        val partialIds =
            state.cleanupExpired(
                nowUnixSecs = unixNow(),
                partialMaxAgeSecs = PARTIAL_RETENTION_SECS.toULong(),
                completionMaxAgeSecs = COMPLETION_RETENTION_SECS.toULong(),
            )
        partialIds.forEach { transferId ->
            runCatching { partialFile(transferId).delete() }
        }
        val orphanCutoff = System.currentTimeMillis() - COMPLETION_RETENTION_SECS * 1000L
        root.listFiles()
            ?.filter { file ->
                file.isFile &&
                    file.name.startsWith("partial-") &&
                    file.name.endsWith(".bin") &&
                    file.lastModified() < orphanCutoff
            }
            ?.forEach { file -> runCatching { file.delete() } }
        commitState()
    }

    private fun recoverPartial(
        partial: File,
        durableOffset: Long,
    ) {
        partial.parentFile?.mkdirs()
        if (!partial.exists()) {
            if (durableOffset != 0L) {
                throw FileTransferStorageUnavailable(
                    "retained partial is shorter than its durable checkpoint",
                )
            }
            partial.createNewFile()
        }
        RandomAccessFile(partial, "rw").use { file ->
            if (file.length() < durableOffset) {
                throw FileTransferStorageUnavailable(
                    "partial file is shorter than its durable checkpoint",
                )
            }
            if (file.length() > durableOffset) {
                file.setLength(durableOffset)
                file.fd.sync()
            }
        }
    }

    private fun displayName(uri: Uri): String {
        resolver.query(
            uri,
            arrayOf(OpenableColumns.DISPLAY_NAME),
            null,
            null,
            null,
        )?.use { cursor ->
            if (cursor.moveToFirst()) {
                val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (index >= 0) {
                    val value = cursor.getString(index)
                    if (!value.isNullOrBlank()) return value
                }
            }
        }
        throw FileTransferStorageUnavailable("selected document has no display name")
    }

    private fun partialFile(transferId: ByteArray): File =
        root.resolve("partial-" + transferId.toHex() + ".bin")

    private fun commitState() {
        val output = stateFile.startWrite()
        try {
            output.write(state.encode())
            output.fd.sync()
            stateFile.finishWrite(output)
        } catch (error: Throwable) {
            stateFile.failWrite(output)
            throw error
        }
    }
}

class AndroidFileTransferSource(
    val uri: Uri,
    val offer: MobileFileTransferOffer,
) {
    override fun toString(): String =
        "AndroidFileTransferSource(uri=[REDACTED], size=" + offer.fileSize() + ")"
}

class AndroidFileTransferSourceReader internal constructor(
    private val input: InputStream,
    private var remaining: Long,
) : AutoCloseable {
    fun readChunk(): ByteArray? {
        if (remaining == 0L) return null
        val requested = min(IO_CHUNK_BYTES.toLong(), remaining).toInt()
        val buffer = ByteArray(requested)
        var offset = 0
        while (offset < requested) {
            val read = input.read(buffer, offset, requested - offset)
            if (read < 0) {
                throw FileTransferStorageUnavailable("source ended before its prepared size")
            }
            if (read == 0) continue
            offset += read
        }
        remaining -= requested
        return buffer
    }

    override fun close() {
        input.close()
    }
}

enum class AndroidReceivePreparationKind {
    READY,
    ALREADY_COMPLETE,
}

data class AndroidReceivePreparation(
    val kind: AndroidReceivePreparationKind,
    val resumeOffset: ULong,
    val receiver: AndroidFileTransferReceiver?,
) {
    companion object {
        fun ready(
            resumeOffset: ULong,
            receiver: AndroidFileTransferReceiver,
        ): AndroidReceivePreparation =
            AndroidReceivePreparation(
                kind = AndroidReceivePreparationKind.READY,
                resumeOffset = resumeOffset,
                receiver = receiver,
            )

        fun alreadyComplete(fileSize: ULong): AndroidReceivePreparation =
            AndroidReceivePreparation(
                kind = AndroidReceivePreparationKind.ALREADY_COMPLETE,
                resumeOffset = fileSize,
                receiver = null,
            )
    }
}

class AndroidFileTransferReceiver internal constructor(
    private val adapter: AndroidFileTransferAdapter,
    private val sourceDeviceId: ByteArray,
    val offer: MobileFileTransferOffer,
    private val destinationUri: Uri,
    private val partial: File,
    initialOffset: Long,
) : AutoCloseable {
    private var file: RandomAccessFile? =
        RandomAccessFile(partial, "rw").apply {
            seek(initialOffset)
        }
    private var offset = initialOffset
    private var terminal = false
    private val expectedSize = offer.fileSize().toSafeLong()

    @Synchronized
    fun write(bytes: ByteArray): ULong {
        check(!terminal) { "file transfer receiver is closed" }
        val active = checkNotNull(file)
        if (bytes.size > IO_CHUNK_BYTES) {
            throw FileTransferStorageUnavailable("file transfer chunk exceeds 64 KiB")
        }
        if (bytes.size.toLong() > expectedSize - offset) {
            failIntegrity()
            throw FileTransferIntegrityFailure()
        }

        var sourceOffset = 0
        while (sourceOffset < bytes.size) {
            val untilCheckpoint =
                if (offset % CHECKPOINT_BYTES == 0L) {
                    CHECKPOINT_BYTES
                } else {
                    CHECKPOINT_BYTES - (offset % CHECKPOINT_BYTES)
                }
            val count =
                min(
                    (bytes.size - sourceOffset).toLong(),
                    untilCheckpoint,
                ).toInt()
            active.write(bytes, sourceOffset, count)
            sourceOffset += count
            offset += count
            if (offset % CHECKPOINT_BYTES == 0L || offset == expectedSize) {
                active.fd.sync()
                adapter.checkpoint(
                    sourceDeviceId = sourceDeviceId,
                    offer = offer,
                    destinationUri = destinationUri,
                    durableOffset = offset,
                )
            }
        }
        return offset.toULong()
    }

    @Synchronized
    fun finish() {
        check(!terminal) { "file transfer receiver is closed" }
        if (offset != expectedSize) {
            failIntegrity()
            throw FileTransferIntegrityFailure()
        }

        file?.fd?.sync()
        file?.close()
        file = null

        val verifier = MobileFileTransferVerifier(offer)
        try {
            FileInputStream(partial).use { input ->
                val buffer = ByteArray(IO_CHUNK_BYTES)
                while (true) {
                    val read = input.read(buffer)
                    if (read < 0) break
                    if (read == 0) continue
                    verifier.update(buffer.copyOf(read))
                }
            }
            verifier.finish()
        } catch (error: Throwable) {
            failIntegrity()
            throw FileTransferIntegrityFailure(error)
        }

        adapter.publish(partial, destinationUri)
        adapter.completed(sourceDeviceId, offer)
        runCatching { partial.delete() }
        terminal = true
    }

    @Synchronized
    fun cancel() {
        if (terminal) return
        file?.fd?.sync()
        file?.close()
        file = null
        terminal = true
    }

    override fun close() {
        cancel()
    }

    private fun failIntegrity() {
        runCatching { file?.setLength(0) }
        runCatching { file?.fd?.sync() }
        runCatching { file?.close() }
        file = null
        runCatching { partial.delete() }
        runCatching { adapter.invalidate(offer) }
        terminal = true
    }
}

class FileTransferStorageUnavailable(
    message: String,
    cause: Throwable? = null,
) : IllegalStateException(message, cause)

class FileTransferSourceChanged :
    IllegalStateException("selected source changed since the transfer was prepared")

class FileTransferIntegrityFailure(
    cause: Throwable? = null,
) : IllegalStateException("received file failed exact size or digest verification", cause)

private fun skipExactly(
    input: InputStream,
    bytes: Long,
) {
    var remaining = bytes
    while (remaining > 0) {
        val skipped = input.skip(remaining)
        if (skipped > 0) {
            remaining -= skipped
            continue
        }
        if (input.read() < 0) {
            throw FileTransferStorageUnavailable("source is shorter than its prepared size")
        }
        remaining -= 1
    }
}

private fun ULong.toSafeLong(): Long {
    require(this <= Long.MAX_VALUE.toULong()) { "file is too large for Android document I/O" }
    return toLong()
}

private fun ByteArray.toHex(): String =
    joinToString(separator = "") { byte -> "%02x".format(byte.toInt() and 0xff) }

private fun unixNow(): ULong = (System.currentTimeMillis() / 1000L).coerceAtLeast(0L).toULong()
