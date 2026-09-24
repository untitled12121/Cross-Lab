package dev.crosslab.android.features.devices

enum class PresenceDisplay(val label: String) {
    UNAVAILABLE("Unavailable"),
    DISCOVERING("Discovering"),
    CONNECTING("Connecting"),
    ONLINE("Online"),
    RECONNECTING("Reconnecting"),
    PAUSED("Paused"),
    FAILED("Discovery failed"),
}

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

enum class PermissionDisplay(val label: String) {
    ALLOW("Allow"),
    DENY("Deny"),
    ASK("Ask"),
}

data class PermissionPresentation(
    val capabilityId: String,
    val operation: String,
    val effect: PermissionDisplay,
)


data class DevicePresentation(
    val ownerId: String?,
    val localDeviceId: String?,
    val peerId: String,
    val trust: TrustDisplay,
    val connectivity: ConnectivityDisplay,
    val session: SessionDisplay,
    val protocol: String?,
    val network: NetworkDisplay,
    val security: SecurityDisplay,
    val metered: Boolean?,
    val capabilityIds: List<String>,
    val capabilityCount: Int,
    val policyRevision: ULong,
    val permissionRules: List<PermissionPresentation>,
)

data class DevicesState(
    val current: DevicePresentation?,
    val presence: PresenceDisplay,
) {
    companion object {
        fun from(snapshot: RuntimeSnapshot): DevicesState {
            val presence =
                when (snapshot.presence) {
                    RuntimePresence.UNAVAILABLE -> PresenceDisplay.UNAVAILABLE
                    RuntimePresence.DISCOVERING -> PresenceDisplay.DISCOVERING
                    RuntimePresence.CONNECTING -> PresenceDisplay.CONNECTING
                    RuntimePresence.ONLINE -> PresenceDisplay.ONLINE
                    RuntimePresence.RECONNECTING -> PresenceDisplay.RECONNECTING
                    RuntimePresence.PAUSED -> PresenceDisplay.PAUSED
                    RuntimePresence.FAILED -> PresenceDisplay.FAILED
                }
            val peerId =
                snapshot.peerDeviceId
                    ?: return DevicesState(current = null, presence = presence)
            return DevicesState(
                current = DevicePresentation(
                    ownerId = snapshot.ownerId?.take(16),
                    localDeviceId = snapshot.localDeviceId?.take(16),
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
                    capabilityIds = snapshot.capabilityIds,
                    capabilityCount = snapshot.capabilityCount,
                    policyRevision = snapshot.policyRevision,
                    permissionRules =
                        snapshot.permissionRules
                            .filter { it.sourceDeviceId == peerId }
                            .map { rule ->
                                PermissionPresentation(
                                    capabilityId = rule.capabilityId,
                                    operation = rule.operation,
                                    effect =
                                        when (rule.effect) {
                                            RuntimePermissionEffect.ALLOW -> PermissionDisplay.ALLOW
                                            RuntimePermissionEffect.DENY -> PermissionDisplay.DENY
                                            RuntimePermissionEffect.ASK -> PermissionDisplay.ASK
                                        },
                                )
                            },
                ),
                presence = presence,
            )
        }
    }
}
