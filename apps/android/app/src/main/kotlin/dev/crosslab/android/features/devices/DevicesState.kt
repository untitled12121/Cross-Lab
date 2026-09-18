package dev.crosslab.android.features.devices

enum class TrustDisplay(val label: String) {
    PENDING("Pending"),
    TRUSTED("Trusted"),
    REVOKED("Revoked"),
}

enum class ConnectivityDisplay(val label: String) {
    CONNECTED("Connected"),
    DISCONNECTED("Disconnected"),
}

enum class SessionDisplay(val label: String) {
    CREATED("Created"),
    AUTHENTICATING("Authenticating"),
    ACTIVE("Active"),
    CLOSING("Closing"),
    CLOSED("Closed"),
    REVOKED("Revoked"),
}

enum class NetworkDisplay(val label: String) {
    LOCAL("Local"),
    TRUSTED("Trusted"),
    REMOTE("Remote"),
    UNAVAILABLE("Unavailable"),
}

enum class SecurityDisplay(val label: String) {
    TEST_ONLY("Test only"),
    AUTHENTICATED("Authenticated"),
    UNAVAILABLE("Unavailable"),
}

data class DevicePresentation(
    val peerId: String,
    val trust: TrustDisplay,
    val connectivity: ConnectivityDisplay,
    val session: SessionDisplay,
    val protocol: String?,
    val network: NetworkDisplay,
    val security: SecurityDisplay,
    val metered: Boolean?,
    val capabilityCount: Int,
)

data class DevicesState(
    val current: DevicePresentation?,
) {
    companion object {
        fun from(snapshot: RuntimeSnapshot): DevicesState {
            val peerId = snapshot.peerDeviceId ?: return DevicesState(current = null)
            return DevicesState(
                current = DevicePresentation(
                    peerId = peerId.take(16),
                    trust = when (snapshot.trust) {
                        RuntimeTrust.PENDING,
                        RuntimeTrust.UNAVAILABLE,
                        -> TrustDisplay.PENDING

                        RuntimeTrust.TRUSTED -> TrustDisplay.TRUSTED
                        RuntimeTrust.REVOKED -> TrustDisplay.REVOKED
                    },
                    connectivity = when (snapshot.connectivity) {
                        RuntimeConnectivity.CONNECTED -> ConnectivityDisplay.CONNECTED
                        RuntimeConnectivity.DISCONNECTED -> ConnectivityDisplay.DISCONNECTED
                    },
                    session = when (snapshot.session) {
                        RuntimeSession.CREATED -> SessionDisplay.CREATED
                        RuntimeSession.AUTHENTICATING -> SessionDisplay.AUTHENTICATING
                        RuntimeSession.ACTIVE -> SessionDisplay.ACTIVE
                        RuntimeSession.CLOSING -> SessionDisplay.CLOSING
                        RuntimeSession.CLOSED,
                        RuntimeSession.UNAVAILABLE,
                        -> SessionDisplay.CLOSED

                        RuntimeSession.REVOKED -> SessionDisplay.REVOKED
                    },
                    protocol = snapshot.protocol?.let { "${it.major}.${it.minor}" },
                    network = when (snapshot.network) {
                        RuntimeNetwork.LOCAL -> NetworkDisplay.LOCAL
                        RuntimeNetwork.TRUSTED -> NetworkDisplay.TRUSTED
                        RuntimeNetwork.REMOTE -> NetworkDisplay.REMOTE
                        RuntimeNetwork.UNAVAILABLE -> NetworkDisplay.UNAVAILABLE
                    },
                    security = when (snapshot.security) {
                        RuntimeSecurity.TEST_ONLY -> SecurityDisplay.TEST_ONLY
                        RuntimeSecurity.AUTHENTICATED -> SecurityDisplay.AUTHENTICATED
                        RuntimeSecurity.UNAVAILABLE -> SecurityDisplay.UNAVAILABLE
                    },
                    metered = snapshot.metered,
                    capabilityCount = snapshot.capabilityCount,
                ),
            )
        }
    }
}
