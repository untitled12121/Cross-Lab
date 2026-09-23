package dev.crosslab.android.features.devices

import uniffi.crosslab_mobile_ffi.MobileConnectivityState
import uniffi.crosslab_mobile_ffi.MobileNetworkClass
import uniffi.crosslab_mobile_ffi.MobilePermissionEffect
import uniffi.crosslab_mobile_ffi.MobilePermissionSnapshot
import uniffi.crosslab_mobile_ffi.MobileRuntimeSnapshot
import uniffi.crosslab_mobile_ffi.MobileSessionState
import uniffi.crosslab_mobile_ffi.MobileTransportSecurity
import uniffi.crosslab_mobile_ffi.MobileTrustState

enum class RuntimePresence {
    UNAVAILABLE,
    DISCOVERING,
    CONNECTING,
    ONLINE,
    RECONNECTING,
    PAUSED,
    FAILED,
}

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

enum class RuntimePermissionEffect {
    ALLOW,
    DENY,
    ASK,
}

data class RuntimePermissionRule(
    val sourceDeviceId: String,
    val capabilityId: String,
    val operation: String,
    val effect: RuntimePermissionEffect,
)


data class RuntimeProtocolVersion(
    val major: Int,
    val minor: Int,
)

data class RuntimeSnapshot(
    val presence: RuntimePresence = RuntimePresence.UNAVAILABLE,
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
    val capabilityIds: List<String>,
    val capabilityCount: Int,
    val policyRevision: ULong,
    val permissionRules: List<RuntimePermissionRule>,
) {
    companion object {
        fun disconnected(): RuntimeSnapshot =
            RuntimeSnapshot(
                presence = RuntimePresence.UNAVAILABLE,
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
                capabilityIds = emptyList(),
                capabilityCount = 0,
                policyRevision = 0uL,
                permissionRules = emptyList(),
            )
    }
}

internal fun MobileRuntimeSnapshot.toRuntimeSnapshot(
    presence: RuntimePresence = RuntimePresence.UNAVAILABLE,
    permissions: MobilePermissionSnapshot? = null,
): RuntimeSnapshot =
    RuntimeSnapshot(
        presence = presence,
        ownerId = ownerId,
        localDeviceId = localDeviceId,
        peerDeviceId = peerDeviceId,
        trust =
            when (trust) {
                MobileTrustState.UNAVAILABLE -> RuntimeTrust.UNAVAILABLE
                MobileTrustState.PENDING -> RuntimeTrust.PENDING
                MobileTrustState.TRUSTED -> RuntimeTrust.TRUSTED
                MobileTrustState.REVOKED -> RuntimeTrust.REVOKED
            },
        connectivity =
            when (connectivity) {
                MobileConnectivityState.CONNECTED -> RuntimeConnectivity.CONNECTED
                MobileConnectivityState.DISCONNECTED -> RuntimeConnectivity.DISCONNECTED
            },
        session =
            when (session) {
                MobileSessionState.UNAVAILABLE -> RuntimeSession.UNAVAILABLE
                MobileSessionState.CREATED -> RuntimeSession.CREATED
                MobileSessionState.AUTHENTICATING -> RuntimeSession.AUTHENTICATING
                MobileSessionState.ACTIVE -> RuntimeSession.ACTIVE
                MobileSessionState.CLOSING -> RuntimeSession.CLOSING
                MobileSessionState.CLOSED -> RuntimeSession.CLOSED
                MobileSessionState.REVOKED -> RuntimeSession.REVOKED
            },
        protocol =
            protocol?.let {
                RuntimeProtocolVersion(
                    major = it.major.toInt(),
                    minor = it.minor.toInt(),
                )
            },
        network =
            when (network) {
                MobileNetworkClass.UNAVAILABLE -> RuntimeNetwork.UNAVAILABLE
                MobileNetworkClass.LOCAL -> RuntimeNetwork.LOCAL
                MobileNetworkClass.TRUSTED -> RuntimeNetwork.TRUSTED
                MobileNetworkClass.REMOTE -> RuntimeNetwork.REMOTE
            },
        security =
            when (transportSecurity) {
                MobileTransportSecurity.UNAVAILABLE -> RuntimeSecurity.UNAVAILABLE
                MobileTransportSecurity.TEST_ONLY -> RuntimeSecurity.TEST_ONLY
                MobileTransportSecurity.AUTHENTICATED -> RuntimeSecurity.AUTHENTICATED
            },
        metered = metered,
        capabilityIds = capabilityIds,
        capabilityCount = capabilityCount.coerceAtMost(Int.MAX_VALUE.toULong()).toInt(),
        policyRevision = permissions?.policyRevision ?: 0uL,
        permissionRules =
            permissions?.rules.orEmpty().map { rule ->
                RuntimePermissionRule(
                    sourceDeviceId = rule.sourceDeviceId,
                    capabilityId = rule.capabilityId,
                    operation = rule.operation,
                    effect =
                        when (rule.effect) {
                            MobilePermissionEffect.ALLOW -> RuntimePermissionEffect.ALLOW
                            MobilePermissionEffect.DENY -> RuntimePermissionEffect.DENY
                            MobilePermissionEffect.ASK -> RuntimePermissionEffect.ASK
                        },
                )
            },
    )
