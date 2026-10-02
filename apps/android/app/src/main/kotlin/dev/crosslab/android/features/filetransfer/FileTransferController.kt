package dev.crosslab.android.features.filetransfer

import android.net.Uri
import java.util.concurrent.CopyOnWriteArraySet

enum class FileTransferDirection { SEND, RECEIVE }

enum class FileTransferStage {
    IDLE, PREPARING, WAITING_PEER, WAITING_DESTINATION, TRANSFERRING,
    FINALIZING, COMPLETED, CANCELLED, FAILED,
}

enum class FileTransferFailure {
    NOT_CONNECTED, NOT_NEGOTIATED, DENIED, SOURCE_CHANGED, STORAGE,
    INTEGRITY, RESOURCE_LIMIT, NETWORK, UNAVAILABLE,
}

data class FileTransferState(
    val available: Boolean,
    val direction: FileTransferDirection? = null,
    val stage: FileTransferStage = FileTransferStage.IDLE,
    val displayName: String? = null,
    val transferredBytes: ULong = 0uL,
    val totalBytes: ULong = 0uL,
    val canCancel: Boolean = false,
    val canRetry: Boolean = false,
    val pendingRequestId: String? = null,
    val resumeAvailable: Boolean = false,
    val failure: FileTransferFailure? = null,
) {
    companion object {
        fun initial(available: Boolean) = FileTransferState(available = available)
    }
}

interface FileTransferPort {
    val fileTransferAvailable: Boolean
        get() = false

    fun fileTransferState() = FileTransferState.initial(fileTransferAvailable)

    fun observeFileTransfer(listener: (FileTransferState) -> Unit): AutoCloseable {
        listener(fileTransferState())
        return AutoCloseable {}
    }

    fun sendFile(uri: Uri, retainReadGrant: Boolean): Boolean = false

    fun retryFile(): Boolean = false

    fun acceptFile(uri: Uri, requestId: String, retainWriteGrant: Boolean): Boolean = false

    fun resumeFile(requestId: String): Boolean = false

    fun declineFile(): Boolean = false

    fun cancelFile(): Boolean = false
}

object UnavailableFileTransferPort : FileTransferPort

class FileTransferController(private val port: FileTransferPort) {
    private val listeners = CopyOnWriteArraySet<(FileTransferState) -> Unit>()

    @Volatile private var current = port.fileTransferState()

    private val subscription = port.observeFileTransfer { state ->
        current = state
        listeners.forEach { it(state) }
    }

    fun state(): FileTransferState = current

    fun observe(listener: (FileTransferState) -> Unit): AutoCloseable {
        listeners += listener
        listener(current)
        return AutoCloseable { listeners -= listener }
    }

    fun send(uri: Uri, retainReadGrant: Boolean) = port.sendFile(uri, retainReadGrant)

    fun retry() = port.retryFile()

    fun accept(uri: Uri, requestId: String, retainWriteGrant: Boolean) =
        port.acceptFile(uri, requestId, retainWriteGrant)

    fun resume(requestId: String) = port.resumeFile(requestId)

    fun decline() = port.declineFile()

    fun cancel() = port.cancelFile()

    fun shutdown() {
        subscription.close()
        listeners.clear()
    }
}
