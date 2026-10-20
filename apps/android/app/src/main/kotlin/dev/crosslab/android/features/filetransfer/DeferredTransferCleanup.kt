package dev.crosslab.android.features.filetransfer

/**
 * A receiver whose asynchronous close was rejected must remain reachable.
 * The service blocks new transfers until its close is admitted to a bounded worker.
 * Access is serialized by the owning service lock.
 */
internal class DeferredTransferCleanup<T : Any> {
    private var pending: T? = null

    fun retain(item: T): Boolean {
        val existing = pending
        if (existing != null && existing !== item) return false
        pending = item
        return true
    }

    fun trySubmit(enqueue: (T) -> Boolean): Boolean {
        val item = pending ?: return true
        if (!enqueue(item)) return false
        pending = null
        return true
    }

    fun hasPending(): Boolean = pending != null
}
