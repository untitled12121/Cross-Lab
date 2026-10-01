package dev.crosslab.android.features.filetransfer

import android.net.Uri
import java.io.InputStream
import kotlin.math.min
import uniffi.crosslab_mobile_ffi.MobileFileTransferOffer

internal const val FILE_TRANSFER_IO_CHUNK_BYTES = 64 * 1024
internal const val FILE_TRANSFER_CHECKPOINT_BYTES = 1_048_576L

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
        val requested = min(FILE_TRANSFER_IO_CHUNK_BYTES.toLong(), remaining).toInt()
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

class FileTransferStorageUnavailable(
    message: String,
    cause: Throwable? = null,
) : IllegalStateException(message, cause)

class FileTransferSourceChanged :
    IllegalStateException("selected source changed since the transfer was prepared")

class FileTransferPreparationCancelled :
    IllegalStateException("file transfer preparation was cancelled")

class FileTransferIntegrityFailure(
    cause: Throwable? = null,
) : IllegalStateException("received file failed exact size or digest verification", cause)

internal fun skipExactly(
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

internal fun ULong.toSafeLong(): Long {
    require(this <= Long.MAX_VALUE.toULong()) { "file is too large for Android document I/O" }
    return toLong()
}

internal fun ByteArray.toHex(): String =
    joinToString(separator = "") { byte -> "%02x".format(byte.toInt() and 0xff) }

internal fun unixNow(): ULong =
    (System.currentTimeMillis() / 1000L).coerceAtLeast(0L).toULong()
