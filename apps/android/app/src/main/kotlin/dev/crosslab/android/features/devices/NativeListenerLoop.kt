package dev.crosslab.android.features.devices

/**
 * Native waits are fallible. If a listener exits while its authenticated agent
 * is still current, the owner-visible session must be invalidated.
 *
 * Normal stop, reconnect, and shutdown first invalidate the agent generation,
 * so their listener exits cannot fault a newer session.
 */
internal fun <E : Any> runNativeListenerLoop(
    isCurrent: () -> Boolean,
    waitNext: () -> E?,
    deliver: (E) -> Unit,
    unexpectedExit: () -> Unit,
) {
    try {
        while (isCurrent()) {
            val event = waitNext() ?: continue
            if (!isCurrent()) return
            deliver(event)
        }
    } finally {
        if (isCurrent()) unexpectedExit()
    }
}
