package dev.crosslab.android.features.devices

enum class RuntimeTrust {
    UNAVAILABLE,
    PENDING,
    TRUSTED,
    REVOKED,
}

enum class RuntimeConnectivity {
    CONNECTED,
    DISCONNECTED,
}

enum class RuntimeSession {
    UNAVAILABLE,
    CREATED,
    AUTHENTICATING,
    ACTIVE,
    CLOSING,
    CLOSED,
    REVOKED,
}

enum class RuntimeNetwork {
    UNAVAILABLE,
    LOCAL,
    TRUSTED,
    REMOTE,
}

enum class RuntimeSecurity {
    UNAVAILABLE,
    TEST_ONLY,
    AUTHENTICATED,
}

data class RuntimeProtocolVersion(
    val major: Int,
    val minor: Int,
)

data class RuntimeSnapshot(
    val ownerId: String?,
    val localDeviceId: String?,
    val peerDeviceId: String?,
    val trust: RuntimeTrust,
    val connectivity: RuntimeConnectivity,
    val session: RuntimeSession,
    val protocol: RuntimeProtocolVersion?,
    val network: RuntimeNetwork,
    val security: RuntimeSecurity,
    val metered: Boolean?,
    val capabilityCount: Int,
) {
    companion object {
        fun disconnected(): RuntimeSnapshot =
            RuntimeSnapshot(
                ownerId = null,
                localDeviceId = null,
                peerDeviceId = null,
                trust = RuntimeTrust.UNAVAILABLE,
                connectivity = RuntimeConnectivity.DISCONNECTED,
                session = RuntimeSession.UNAVAILABLE,
                protocol = null,
                network = RuntimeNetwork.UNAVAILABLE,
                security = RuntimeSecurity.UNAVAILABLE,
                metered = null,
                capabilityCount = 0,
            )
    }
}
