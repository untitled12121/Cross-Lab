package dev.crosslab.android.features.filetransfer

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import android.util.AtomicFile
import java.io.File
import java.io.FileInputStream
import java.io.RandomAccessFile
import uniffi.crosslab_mobile_ffi.MobileFileTransferHasher
import uniffi.crosslab_mobile_ffi.MobileFileTransferOffer
import uniffi.crosslab_mobile_ffi.MobileFileTransferRecoveryKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferState

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
        isCancelled: () -> Boolean = { false },
    ): AndroidFileTransferSource {
        val displayName = displayName(uri)
        val hasher = MobileFileTransferHasher()
        resolver.openInputStream(uri)?.use { input ->
            val buffer = ByteArray(FILE_TRANSFER_IO_CHUNK_BYTES)
            while (true) {
                if (isCancelled()) throw FileTransferPreparationCancelled()
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
        offer.fileSize().toSafeLong()
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
        offer.fileSize().toSafeLong()
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
        return AndroidReceivePreparation.ready(
            durableOffset,
            AndroidFileTransferReceiver(
                adapter = this,
                sourceDeviceId = sourceDeviceId.copyOf(),
                offer = offer,
                destinationUri = resolvedUri,
                partial = partial,
                initialOffset = durableOffset.toSafeLong(),
            ),
        )
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
                val buffer = ByteArray(FILE_TRANSFER_IO_CHUNK_BYTES)
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
        val expired =
            state.cleanupExpired(
                nowUnixSecs = unixNow(),
                partialMaxAgeSecs = PARTIAL_RETENTION_SECS.toULong(),
                completionMaxAgeSecs = COMPLETION_RETENTION_SECS.toULong(),
            )
        expired.forEach { entry ->
            runCatching { partialFile(entry.transferId).delete() }
        }

        val orphanCutoff = System.currentTimeMillis() - PARTIAL_RETENTION_SECS * 1000L
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
            if (!partial.createNewFile()) {
                throw FileTransferStorageUnavailable("partial file could not be created")
            }
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
