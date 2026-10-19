package dev.crosslab.android.features.filetransfer

/**
 * A failed durable write must never make a candidate state visible to readers.
 * The caller serializes mutations and provides a deep snapshot.
 */
internal fun <State, Result> persistThenApply(
    current: State,
    snapshot: (State) -> State,
    edit: (State) -> Result,
    persist: (State) -> Unit,
): Pair<State, Result> {
    val candidate = snapshot(current)
    val result = edit(candidate)
    persist(candidate)
    return candidate to result
}
