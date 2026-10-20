package dev.crosslab.android.features.filetransfer

/**
 * Retains receiver ownership through asynchronous close, not just queue admission.
 * The owning service serializes all access under its lifecycle lock.
 */
internal class DeferredTransferCleanup<T : Any> {
    private var pending: T? = null
    private var submitted = false

    fun retain(item: T): Boolean {
        val existing = pending
        if (existing != null && existing !== item) return false
        pending = item
        return true
    }

    /** True means accepted (or already accepted), not that cleanup has finished. */
    fun trySubmit(enqueue: (T) -> Boolean): Boolean {
        val item = pending ?: return true
        if (submitted) return true
        if (!enqueue(item)) return false
        submitted = true
        return true
    }

    fun complete(item: T): Boolean {
        if (pending !== item || !submitted) return false
        pending = null
        submitted = false
        return true
    }

    /** Only an unsubmitted receiver may be handed to a terminal close fallback. */
    fun takeUnsubmitted(): T? {
        if (submitted) return null
        val item = pending
        pending = null
        return item
    }

    fun hasPending(): Boolean = pending != null
}
