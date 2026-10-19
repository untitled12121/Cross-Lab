package dev.crosslab.android.features.owner

import dev.crosslab.android.features.devices.RuntimeConnectivity
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.features.devices.RuntimeLifecycle
import dev.crosslab.android.features.devices.RuntimePermissionEffect
import dev.crosslab.android.features.devices.RuntimePermissionRule
import dev.crosslab.android.features.devices.RuntimeSecurity
import dev.crosslab.android.features.devices.RuntimeSession
import dev.crosslab.android.features.devices.RuntimeSnapshot
import dev.crosslab.android.features.devices.RuntimeTrust
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class OwnerPeerPermissionsTest {
    private val peer = "a".repeat(64)

    private fun secureSession() =
        RuntimeControllerState.initial().copy(
            lifecycle = RuntimeLifecycle.RUNNING,
            snapshot =
                RuntimeSnapshot.disconnected().copy(
                    peerDeviceId = peer,
                    trust = RuntimeTrust.TRUSTED,
                    session = RuntimeSession.ACTIVE,
                    connectivity = RuntimeConnectivity.CONNECTED,
                    security = RuntimeSecurity.AUTHENTICATED,
                    capabilityIds =
                        listOf(
                            "clipboard.read",
                            "clipboard.write",
                            "files.transfer",
                            "notifications.read",
                            "unapproved.capability",
                        ),
                ),
        )

    @Test
    fun onlyNegotiatedCanonicalOperationsAreDisplayedAndDefaultToDeny() {
        val state = OwnerPeerPermissionState.from(secureSession())
        assertEquals(peer, state.peerId)
        assertEquals(4, state.operations.size)
        assertEquals(
            listOf("get", "set", "receive", "subscribe"),
            state.operations.map { it.operation },
        )
        assertTrue(state.operations.all { it.effect == null })
        assertTrue(state.operations.all { it.policyLabel().contains("Default deny") })
    }

    @Test
    fun permissionsAreMatchedByFullExactPeerAndOperation() {
        val rules =
            listOf(
                RuntimePermissionRule("a".repeat(63) + "b", "clipboard.read", "get", RuntimePermissionEffect.ALLOW),
                RuntimePermissionRule(peer, "clipboard.read", "set", RuntimePermissionEffect.ALLOW),
                RuntimePermissionRule(peer, "clipboard.read", "get", RuntimePermissionEffect.DENY),
            )
        val state =
            OwnerPeerPermissionState.from(
                secureSession().copy(snapshot = secureSession().snapshot.copy(permissionRules = rules)),
            )
        assertEquals(RuntimePermissionEffect.DENY, state.operations.first().effect)
        assertNull(state.operations[1].effect)
    }

    @Test
    fun noControlsWithoutTrustedConnectedAuthenticatedActiveSession() {
        val state = secureSession()
        val unsafe =
            listOf(
                state.copy(lifecycle = RuntimeLifecycle.STOPPED),
                state.copy(networkAvailable = false),
                state.copy(snapshot = state.snapshot.copy(session = RuntimeSession.CLOSED)),
                state.copy(snapshot = state.snapshot.copy(trust = RuntimeTrust.REVOKED)),
                state.copy(snapshot = state.snapshot.copy(connectivity = RuntimeConnectivity.DISCONNECTED)),
                state.copy(snapshot = state.snapshot.copy(security = RuntimeSecurity.TEST_ONLY)),
                state.copy(snapshot = state.snapshot.copy(peerDeviceId = null)),
            )
        assertTrue(unsafe.all { OwnerPeerPermissionState.from(it).peerId == null })
        assertTrue(unsafe.all { OwnerPeerPermissionState.from(it).operations.isEmpty() })
    }
}
