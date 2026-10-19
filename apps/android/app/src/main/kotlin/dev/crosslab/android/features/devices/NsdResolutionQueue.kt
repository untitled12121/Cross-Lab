package dev.crosslab.android.features.devices

/** Serialize legacy Android DNS-SD resolution and reject stale service callbacks. */
internal data class NsdResolveRequest<T>(val instance: String, val value: T)

internal data class NsdResolveCompletion<T>(
    val accepted: Boolean,
    val next: NsdResolveRequest<T>?,
)

/** Caller owns synchronization; active requests retain a slot until the OS replies. */
internal class NsdResolutionQueue<T>(private val maxCandidates: Int) {
    private val pending = LinkedHashMap<String, T>()
    private var inFlight: NsdResolveRequest<T>? = null
    private var lostInFlight = false
    private var closed = false

    fun discovered(
        instance: String,
        value: T,
        knownCandidates: Int,
    ): NsdResolveRequest<T>? {
        if (closed || maxCandidates <= 0 ||
            pending.containsKey(instance) ||
            (inFlight?.instance == instance && !lostInFlight)
        ) {
            return null
        }
        if (knownCandidates + pending.size + (if (inFlight != null) 1 else 0) >= maxCandidates) {
            return null
        }
        pending[instance] = value
        return dispatch()
    }

    fun lost(instance: String) {
        pending.remove(instance)
        if (inFlight?.instance == instance) lostInFlight = true
    }

    fun finished(request: NsdResolveRequest<T>): NsdResolveCompletion<T> {
        // Names can be reused after a lost/reappeared event. Only this exact
        // request's listener may complete the current in-flight resolution.
        if (closed || inFlight !== request) {
            return NsdResolveCompletion(false, null)
        }
        val accepted = !lostInFlight
        inFlight = null
        lostInFlight = false
        return NsdResolveCompletion(accepted, dispatch())
    }

    fun close() {
        closed = true
        pending.clear()
        inFlight = null
    }

    private fun dispatch(): NsdResolveRequest<T>? {
        if (closed || inFlight != null) return null
        val entry = pending.entries.firstOrNull() ?: return null
        pending.remove(entry.key)
        return NsdResolveRequest(entry.key, entry.value).also { inFlight = it }
    }
}
