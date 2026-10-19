package dev.crosslab.android.features.owner

import dev.crosslab.android.features.devices.RuntimeConnectivity
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.features.devices.RuntimeLifecycle
import dev.crosslab.android.features.devices.RuntimePermissionEffect
import dev.crosslab.android.features.devices.RuntimeSecurity
import dev.crosslab.android.features.devices.RuntimeSession
import dev.crosslab.android.features.devices.RuntimeTrust

/** Only audited Phase 2 operations; never construct authorization keys from display strings. */
internal data class OwnerPeerOperation(
    val title: String,
    val capabilityId: String,
    val operation: String,
    val effect: RuntimePermissionEffect?,
)

internal data class OwnerPeerPermissionState(
    val peerId: String?,
    val operations: List<OwnerPeerOperation>,
) {
    companion object {
        fun from(runtime: RuntimeControllerState): OwnerPeerPermissionState {
            val snapshot = runtime.snapshot
            val peer = snapshot.peerDeviceId?.takeIf { it.isNotBlank() }
            if (runtime.lifecycle != RuntimeLifecycle.RUNNING ||
                !runtime.networkAvailable ||
                peer == null ||
                snapshot.trust != RuntimeTrust.TRUSTED ||
                snapshot.session != RuntimeSession.ACTIVE ||
                snapshot.connectivity != RuntimeConnectivity.CONNECTED ||
                snapshot.security != RuntimeSecurity.AUTHENTICATED
            ) {
                return OwnerPeerPermissionState(null, emptyList())
            }
            val negotiated = snapshot.capabilityIds.toSet()
            val operations =
                SUPPORTED_PEER_OPERATIONS
                    .filter { it.capabilityId in negotiated }
                    .map { spec ->
                        val effect =
                            snapshot.permissionRules
                                .firstOrNull {
                                    it.sourceDeviceId == peer &&
                                        it.capabilityId == spec.capabilityId &&
                                        it.operation == spec.operation
                                }
                                ?.effect
                        spec.copy(effect = effect)
                    }
            return OwnerPeerPermissionState(peer, operations)
        }
    }
}

private val SUPPORTED_PEER_OPERATIONS =
    listOf(
        OwnerPeerOperation("Peer reads this device clipboard", "clipboard.read", "get", null),
        OwnerPeerOperation("Peer writes this device clipboard", "clipboard.write", "set", null),
        OwnerPeerOperation("Peer requests file reception", "files.transfer", "receive", null),
        OwnerPeerOperation("Peer requests mirrored notifications", "notifications.read", "subscribe", null),
    )

internal fun OwnerPeerOperation.policyLabel(): String =
    when (effect) {
        RuntimePermissionEffect.ALLOW -> "Allowed for this peer"
        RuntimePermissionEffect.DENY -> "Denied for this peer"
        RuntimePermissionEffect.ASK -> "Ask (not automatically allowed)"
        null -> "Default deny (no exact rule)"
    }
