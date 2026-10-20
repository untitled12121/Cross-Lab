package dev.crosslab.android.features.filetransfer

import dev.crosslab.android.features.observers.notifyObservers

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
    private val lock = Any()
    private val closed = java.util.concurrent.atomic.AtomicBoolean(false)
    private var nativeRevision = 0L

    @Volatile
    private var current = runCatching { port.fileTransferState() }.getOrElse {
        unavailableState()
    }

    private val subscription: AutoCloseable =
        runCatching {
            port.observeFileTransfer { state ->
                val update = synchronized(lock) {
                    if (closed.get()) null
                    else {
                        nativeRevision++
                        current = state
                        state
                    }
                }
                update?.let { notifyObservers(listeners, it) }
            }
        }.getOrElse {
            val failed = synchronized(lock) {
                nativeRevision++
                current = unavailableState()
                current
            }
            notifyObservers(listeners, failed)
            AutoCloseable {}
        }

    fun state(): FileTransferState = current

    fun observe(listener: (FileTransferState) -> Unit): AutoCloseable {
        if (!closed.get()) listeners += listener
        runCatching { listener(current) }
        return AutoCloseable { listeners -= listener }
    }

    fun send(uri: Uri, retainReadGrant: Boolean): Boolean =
        execute { port.sendFile(uri, retainReadGrant) }

    fun retry(): Boolean = execute { port.retryFile() }

    fun accept(uri: Uri, requestId: String, retainWriteGrant: Boolean): Boolean =
        execute { port.acceptFile(uri, requestId, retainWriteGrant) }

    fun resume(requestId: String): Boolean = execute { port.resumeFile(requestId) }

    fun decline(): Boolean = execute { port.declineFile() }

    fun cancel(): Boolean = execute { port.cancelFile() }

    private fun execute(action: () -> Boolean): Boolean {
        // Never hold controller state locks while crossing native/adapter boundaries.
        val before = synchronized(lock) {
            if (closed.get()) return false
            nativeRevision
        }
        return runCatching { action() }.getOrElse {
            // An intervening native publication is newer evidence; never overwrite it.
            val failed = synchronized(lock) {
                if (closed.get() || nativeRevision != before) null
                else {
                    nativeRevision++
                    current = unavailableState()
                    current
                }
            }
            failed?.let { notifyObservers(listeners, it) }
            false
        }
    }

    fun shutdown() {
        if (!closed.compareAndSet(false, true)) return
        runCatching { subscription.close() }
        val stopped = synchronized(lock) {
            nativeRevision++
            current = FileTransferState.initial(false)
            current
        }
        notifyObservers(listeners, stopped)
        listeners.clear()
    }

    private fun unavailableState() = FileTransferState(
        available = false,
        stage = FileTransferStage.FAILED,
        failure = FileTransferFailure.UNAVAILABLE,
    )
}
