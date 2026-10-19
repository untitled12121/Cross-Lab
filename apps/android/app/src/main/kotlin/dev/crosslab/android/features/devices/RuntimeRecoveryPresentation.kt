package dev.crosslab.android.features.devices

import dev.crosslab.android.components.ui.StatusTone

/** A UI-only recovery command; authorization and session control remain native. */
internal enum class RuntimeRecoveryAction(val label: String) {
    NONE(""),
    RETRY("Retry runtime"),
    RECONNECT("Reconnect"),
    DISCONNECT("Disconnect"),
}

/**
 * Runtime lifecycle and authenticated session proof are separate states.
 * Never promote discovery, durable trust, or mere presence to an authenticated session.
 */
internal data class RuntimeRecoveryPresentation(
    val label: String,
    val detail: String,
    val tone: StatusTone,
    val action: RuntimeRecoveryAction = RuntimeRecoveryAction.NONE,
    val actionEnabled: Boolean = false,
) {
    companion object {
        fun from(runtime: RuntimeControllerState, devices: DevicesState): RuntimeRecoveryPresentation {
            val snapshot = runtime.snapshot
            if (runtime.lifecycle == RuntimeLifecycle.SHUTDOWN) {
                return RuntimeRecoveryPresentation(
                    label = "Runtime shut down",
                    detail = "Cross-Lab stopped the native runtime for safety. Restart the app to recover. Peer actions are unavailable.",
                    tone = StatusTone.CRITICAL,
                )
            }
            if (runtime.lifecycle == RuntimeLifecycle.STOPPED) {
                val failed = snapshot.presence == RuntimePresence.FAILED
                return RuntimeRecoveryPresentation(
                    label = if (failed) "Runtime failed" else "Runtime paused",
                    detail =
                        if (failed) {
                            "Native session recovery failed and peer access was cleared. Retry the runtime after checking local connectivity."
                        } else {
                            "The runtime is stopped. Retry to resume discovery of owner-approved devices."
                        },
                    tone = if (failed) StatusTone.CRITICAL else StatusTone.NEUTRAL,
                    action = if (runtime.peerControlAvailable) RuntimeRecoveryAction.RETRY
                        else RuntimeRecoveryAction.NONE,
                    actionEnabled = runtime.peerControlAvailable && runtime.networkAvailable,
                )
            }
            if (!runtime.networkAvailable) {
                return RuntimeRecoveryPresentation(
                    label = "LAN unavailable",
                    detail = "No supported local network is available. Peer actions are paused until Wi-Fi or Ethernet connectivity returns.",
                    tone = StatusTone.NEUTRAL,
                )
            }
            if (snapshot.trust == RuntimeTrust.REVOKED ||
                snapshot.session == RuntimeSession.REVOKED
            ) {
                return RuntimeRecoveryPresentation(
                    label = "Peer revoked",
                    detail = "This peer's trust was revoked. Reconnecting cannot restore it; pair a different owner-approved device.",
                    tone = StatusTone.CRITICAL,
                )
            }
            val authenticated =
                snapshot.presence == RuntimePresence.ONLINE &&
                    snapshot.peerDeviceId != null &&
                    snapshot.trust == RuntimeTrust.TRUSTED &&
                    snapshot.connectivity == RuntimeConnectivity.CONNECTED &&
                    snapshot.session == RuntimeSession.ACTIVE &&
                    snapshot.security == RuntimeSecurity.AUTHENTICATED
            if (authenticated) {
                return RuntimeRecoveryPresentation(
                    label = "Authenticated session",
                    detail = "A trusted peer has a live authenticated session. Disconnect ends this session without revoking durable trust.",
                    tone = StatusTone.ACCENT,
                    action = if (runtime.peerControlAvailable) RuntimeRecoveryAction.DISCONNECT
                        else RuntimeRecoveryAction.NONE,
                    actionEnabled = runtime.peerControlAvailable,
                )
            }
            val reconnect =
                runtime.peerControlAvailable &&
                    (devices.offersReconnect() ||
                        snapshot.presence == RuntimePresence.ONLINE)
            return when (snapshot.presence) {
                RuntimePresence.FAILED ->
                    RuntimeRecoveryPresentation(
                        label = "Discovery failed",
                        detail = "Local discovery or authentication failed. Reconnect to establish a fresh, verified session.",
                        tone = StatusTone.CRITICAL,
                        action = if (reconnect) RuntimeRecoveryAction.RECONNECT
                            else RuntimeRecoveryAction.NONE,
                        actionEnabled = reconnect,
                    )
                RuntimePresence.PAUSED ->
                    RuntimeRecoveryPresentation(
                        label = "Connection paused",
                        detail = "Automatic connection attempts are paused. Reconnect to resume fresh authentication.",
                        tone = StatusTone.NEUTRAL,
                        action = if (reconnect) RuntimeRecoveryAction.RECONNECT
                            else RuntimeRecoveryAction.NONE,
                        actionEnabled = reconnect,
                    )
                RuntimePresence.ONLINE ->
                    RuntimeRecoveryPresentation(
                        label = "Session not verified",
                        detail = "The reported peer presence is not an authenticated session. Peer capabilities remain blocked. Reconnect to verify again.",
                        tone = StatusTone.CRITICAL,
                        action = if (reconnect) RuntimeRecoveryAction.RECONNECT
                            else RuntimeRecoveryAction.NONE,
                        actionEnabled = reconnect,
                    )
                RuntimePresence.RECONNECTING ->
                    RuntimeRecoveryPresentation(
                        label = "Reconnecting",
                        detail = "The previous session ended. Waiting for a fresh authenticated session before enabling peer actions.",
                        tone = StatusTone.NEUTRAL,
                    )
                RuntimePresence.CONNECTING ->
                    RuntimeRecoveryPresentation(
                        label = "Authenticating",
                        detail = "Cross-Lab is validating peer trust and a fresh session proof. Capabilities are not yet available.",
                        tone = StatusTone.NEUTRAL,
                    )
                RuntimePresence.DISCOVERING ->
                    RuntimeRecoveryPresentation(
                        label = "Discovering",
                        detail = "Looking for trusted devices on the LAN. Discovery alone does not authenticate a peer.",
                        tone = StatusTone.NEUTRAL,
                    )
                RuntimePresence.UNAVAILABLE ->
                    RuntimeRecoveryPresentation(
                        label = "No active session",
                        detail = "No trusted peer is authenticated. Pair a device or reconnect to begin discovery.",
                        tone = StatusTone.NEUTRAL,
                        action = if (reconnect) RuntimeRecoveryAction.RECONNECT
                            else RuntimeRecoveryAction.NONE,
                        actionEnabled = reconnect,
                    )
            }
        }
    }
}
