package dev.crosslab.android.features.clipboard

import java.util.concurrent.Semaphore
import java.util.concurrent.atomic.AtomicBoolean

/** Bound pending clipboard UI callbacks independently of the native event queue. */
internal class ClipboardUiAdmission(capacity: Int) {
    private val permits = Semaphore(capacity)

    init {
        require(capacity > 0)
    }

    fun dispatch(post: ((() -> Unit) -> Boolean), action: () -> Unit): Boolean {
        if (!permits.tryAcquire()) return false
        val owned = AtomicBoolean(true)
        val release = {
            if (owned.compareAndSet(true, false)) permits.release()
        }
        val queued = runCatching {
            post {
                try {
                    action()
                } finally {
                    release()
                }
            }
        }.getOrDefault(false)
        if (!queued) release()
        return queued
    }
}

/** A native callback fault must not escape to the Android main thread. */
internal fun invokeClipboardCallback(
    action: () -> Unit,
    onFailure: () -> Unit,
) {
    runCatching(action).onFailure {
        // The fallback only schedules a bounded native error response.
        runCatching(onFailure)
    }
}
