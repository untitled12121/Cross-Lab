package dev.crosslab.android.features.devices

/**
 * Runtime-only peer/session token. Keep the raw authenticated session identifier
 * out of Compose state, app diagnostics and stable storage.
 */
internal class NativeSessionBinding(
    private val peer: String,
    private val session: String,
) {
    fun matches(other: NativeSessionBinding?): Boolean =
        other != null && peer == other.peer && session == other.session

    override fun toString(): String = "NativeSessionBinding([redacted])"
}

internal fun authenticatedSessionBinding(
    snapshot: RuntimeSnapshot,
    sessionId: String?,
): NativeSessionBinding? {
    val peer = snapshot.peerDeviceId ?: return null
    if (peer.isBlank() ||
        snapshot.presence != RuntimePresence.ONLINE ||
        snapshot.trust != RuntimeTrust.TRUSTED ||
        snapshot.connectivity != RuntimeConnectivity.CONNECTED ||
        snapshot.session != RuntimeSession.ACTIVE ||
        snapshot.security != RuntimeSecurity.AUTHENTICATED
    ) return null
    if (sessionId == null || sessionId.length != 64 ||
        !sessionId.all { it in '0'..'9' || it in 'a'..'f' }
    ) {
        return null
    }
    return NativeSessionBinding(peer, sessionId)
}
