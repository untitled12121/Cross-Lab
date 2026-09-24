package dev.crosslab.android.features.devices

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DevicesStateTest {
    @Test
    fun disconnectedSnapshotDoesNotInventADevice() {
        val state = DevicesState.from(RuntimeSnapshot.disconnected())

        assertNull(state.current)
    }

    @Test
    fun reconnectingPresenceDoesNotInventADevice() {
        val state =
            DevicesState.from(
                RuntimeSnapshot.disconnected().copy(
                    presence = RuntimePresence.RECONNECTING,
                ),
            )

        assertNull(state.current)
        assertEquals(PresenceDisplay.RECONNECTING, state.presence)
    }

    @Test
    fun connectedTrustedSnapshotMapsToPresentationState() {
        val state = DevicesState.from(
            RuntimeSnapshot(
                ownerId = "aaaaaaaaaaaaaaaa".repeat(4),
                localDeviceId = "bbbbbbbbbbbbbbbb".repeat(4),
                peerDeviceId = "0123456789abcdef".repeat(4),
                trust = RuntimeTrust.TRUSTED,
                connectivity = RuntimeConnectivity.CONNECTED,
                session = RuntimeSession.ACTIVE,
                protocol = RuntimeProtocolVersion(1, 4),
                network = RuntimeNetwork.LOCAL,
                security = RuntimeSecurity.AUTHENTICATED,
                metered = false,
                capabilityIds = listOf("files.transfer", "clipboard.read"),
                capabilityCount = 2,
                policyRevision = 4uL,
                permissionRules =
                    listOf(
                        RuntimePermissionRule(
                            sourceDeviceId = "0123456789abcdef".repeat(4),
                            capabilityId = "files.transfer",
                            operation = "receive",
                            effect = RuntimePermissionEffect.ALLOW,
                        ),
                        RuntimePermissionRule(
                            sourceDeviceId = "ffffffffffffffff".repeat(4),
                            capabilityId = "clipboard.read",
                            operation = "read",
                            effect = RuntimePermissionEffect.DENY,
                        ),
                    ),
            ),
        )

        val device = checkNotNull(state.current)
        assertEquals("aaaaaaaaaaaaaaaa", device.ownerId)
        assertEquals("bbbbbbbbbbbbbbbb", device.localDeviceId)
        assertEquals("0123456789abcdef", device.peerId)
        assertEquals(TrustDisplay.TRUSTED, device.trust)
        assertEquals(ConnectivityDisplay.CONNECTED, device.connectivity)
        assertEquals(SessionDisplay.ACTIVE, device.session)
        assertEquals("1.4", device.protocol)
        assertEquals(NetworkDisplay.LOCAL, device.network)
        assertEquals(SecurityDisplay.AUTHENTICATED, device.security)
        assertEquals(listOf("files.transfer", "clipboard.read"), device.capabilityIds)
        assertEquals(2, device.capabilityCount)
        assertEquals(4uL, device.policyRevision)
        assertEquals(1, device.permissionRules.size)
        assertEquals("files.transfer", device.permissionRules.single().capabilityId)
        assertEquals("receive", device.permissionRules.single().operation)
        assertEquals(PermissionDisplay.ALLOW, device.permissionRules.single().effect)
    }

    @Test
    fun revokedSnapshotRemainsRevokedAndDisconnected() {
        val state = DevicesState.from(
            RuntimeSnapshot(
                ownerId = "aaaaaaaaaaaaaaaa".repeat(4),
                localDeviceId = "bbbbbbbbbbbbbbbb".repeat(4),
                peerDeviceId = "fedcba9876543210".repeat(4),
                trust = RuntimeTrust.REVOKED,
                connectivity = RuntimeConnectivity.DISCONNECTED,
                session = RuntimeSession.REVOKED,
                protocol = null,
                network = RuntimeNetwork.REMOTE,
                security = RuntimeSecurity.AUTHENTICATED,
                metered = null,
                capabilityCount = 0,
            ),
        )

        val device = checkNotNull(state.current)
        assertEquals(TrustDisplay.REVOKED, device.trust)
        assertEquals(ConnectivityDisplay.DISCONNECTED, device.connectivity)
        assertEquals(SessionDisplay.REVOKED, device.session)
    }
}
