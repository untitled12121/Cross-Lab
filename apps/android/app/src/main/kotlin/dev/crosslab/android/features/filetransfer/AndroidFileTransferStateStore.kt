package dev.crosslab.android.features.filetransfer

import android.content.Context
import android.util.AtomicFile

private const val MAX_FILE_TRANSFER_STATE_BYTES = 1024 * 1024

class AndroidFileTransferStateStore(context: Context) {
    private val stateFile =
        AtomicFile(
            context.noBackupFilesDir
                .resolve("crosslab/file-transfer/state-v1.bin")
                .apply { parentFile?.mkdirs() },
        )

    @Synchronized
    fun load(): ByteArray? {
        if (!stateFile.baseFile.isFile) return null
        val bytes =
            runCatching { stateFile.readFully() }
                .getOrElse { throw FileTransferStateUnavailable("file transfer state is unreadable") }
        if (bytes.size > MAX_FILE_TRANSFER_STATE_BYTES) {
            throw FileTransferStateUnavailable("file transfer state is oversized")
        }
        return bytes
    }

    @Synchronized
    fun commit(encoded: ByteArray) {
        if (encoded.size > MAX_FILE_TRANSFER_STATE_BYTES) {
            throw FileTransferStateUnavailable("file transfer state is oversized")
        }

        val output = stateFile.startWrite()
        try {
            output.write(encoded)
            output.fd.sync()
            stateFile.finishWrite(output)
        } catch (error: Throwable) {
            stateFile.failWrite(output)
            throw error
        }
    }

    @Synchronized
    fun wipe() {
        stateFile.delete()
    }
}

class FileTransferStateUnavailable(
    message: String,
) : IllegalStateException(message)
