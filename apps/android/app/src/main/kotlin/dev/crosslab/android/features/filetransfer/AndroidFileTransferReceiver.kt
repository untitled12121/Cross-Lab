package dev.crosslab.android.features.filetransfer

import android.net.Uri
import java.io.File
import java.io.FileInputStream
import java.io.RandomAccessFile
import kotlin.math.min
import uniffi.crosslab_mobile_ffi.MobileFileTransferOffer
import uniffi.crosslab_mobile_ffi.MobileFileTransferVerifier

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
        if (bytes.size > FILE_TRANSFER_IO_CHUNK_BYTES) {
            throw FileTransferStorageUnavailable("file transfer chunk exceeds 64 KiB")
        }
        if (bytes.size.toLong() > expectedSize - offset) {
            failIntegrity()
            throw FileTransferIntegrityFailure()
        }

        var sourceOffset = 0
        while (sourceOffset < bytes.size) {
            val untilCheckpoint =
                if (offset % FILE_TRANSFER_CHECKPOINT_BYTES == 0L) {
                    FILE_TRANSFER_CHECKPOINT_BYTES
                } else {
                    FILE_TRANSFER_CHECKPOINT_BYTES - (offset % FILE_TRANSFER_CHECKPOINT_BYTES)
                }
            val count =
                min(
                    (bytes.size - sourceOffset).toLong(),
                    untilCheckpoint,
                ).toInt()
            active.write(bytes, sourceOffset, count)
            sourceOffset += count
            offset += count
            if (
                offset % FILE_TRANSFER_CHECKPOINT_BYTES == 0L ||
                offset == expectedSize
            ) {
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

        val active = checkNotNull(file)
        file = null
        try {
            active.fd.sync()
        } finally {
            active.close()
        }

        val verifier = MobileFileTransferVerifier(offer)
        FileInputStream(partial).use { input ->
            val buffer = ByteArray(FILE_TRANSFER_IO_CHUNK_BYTES)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                if (read == 0) continue
                try {
                    verifier.update(buffer.copyOf(read))
                } catch (error: Throwable) {
                    failIntegrity()
                    throw FileTransferIntegrityFailure(error)
                }
            }
        }
        try {
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
        val active = file
        file = null
        runCatching { active?.fd?.sync() }
        runCatching { active?.close() }
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
