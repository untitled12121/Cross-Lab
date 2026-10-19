package dev.crosslab.android.features.observers

import java.util.concurrent.CopyOnWriteArraySet

/** An observer cannot interrupt security state transitions or another subscriber. */
internal fun <State> notifyObservers(
    listeners: CopyOnWriteArraySet<(State) -> Unit>,
    state: State,
) {
    listeners.forEach { listener -> runCatching { listener(state) } }
}
