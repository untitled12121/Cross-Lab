package dev.crosslab.android.features.observers

import java.util.concurrent.atomic.AtomicBoolean

/**
 * The UI owner, not a native agent callback, decides whether a queued screen update
 * is still permitted. Shutdown invalidates both future and already-queued deliveries.
 */
internal class UiCallbackGate(private val enqueue: (() -> Unit) -> Unit) : AutoCloseable {
    private val closed = AtomicBoolean(false)

    fun dispatch(update: () -> Unit) {
        if (closed.get()) return
        runCatching {
            enqueue {
                if (!closed.get()) runCatching { update() }
            }
        }
    }

    override fun close() {
        closed.set(true)
    }
}

/** Closing one misbehaving observer must never leave remaining observers registered. */
internal fun closeObservers(subscriptions: Iterable<AutoCloseable?>) {
    subscriptions.forEach { subscriber ->
        runCatching { subscriber?.close() }
    }
}
