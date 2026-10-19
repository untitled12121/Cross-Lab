package dev.crosslab.android.features.devices

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DevicesStateTest {
    @Test
    fun nativeCapabilityActionsRequireVerifiedOnlineAuthenticatedSession() {
        val active = RuntimeSnapshot.disconnected().copy(
            presence = RuntimePresence.ONLINE,
            peerDeviceId = "trusted-device-id",
            trust = RuntimeTrust.TRUSTED,
            connectivity = RuntimeConnectivity.CONNECTED,
            session = RuntimeSession.ACTIVE,
            security = RuntimeSecurity.AUTHENTICATED,
            capabilityIds = listOf("files.transfer", "clipboard.read"),
        )
        assertTrue(active.hasAuthenticatedCapability("files.transfer"))
        assertFalse(active.hasAuthenticatedCapability("clipboard.write"))

        val visible = checkNotNull(DevicesState.from(active).current)
        assertTrue(visible.hasAuthenticatedSession(PresenceDisplay.ONLINE))
        assertTrue(visible.hasAuthenticatedCapability("files.transfer", PresenceDisplay.ONLINE))
        assertTrue(visible.hasAuthenticatedCapability("clipboard.read", PresenceDisplay.ONLINE))
        assertFalse(visible.hasAuthenticatedCapability("clipboard.write", PresenceDisplay.ONLINE))
        assertFalse(visible.hasAuthenticatedSession(PresenceDisplay.RECONNECTING))
        assertFalse(visible.hasAuthenticatedCapability("files.transfer", PresenceDisplay.RECONNECTING))
        assertFalse(visible.copy(trust = TrustDisplay.REVOKED)
            .hasAuthenticatedCapability("files.transfer", PresenceDisplay.ONLINE))
        assertFalse(visible.copy(security = SecurityDisplay.TEST_ONLY)
            .hasAuthenticatedCapability("files.transfer", PresenceDisplay.ONLINE))
        assertFalse(visible.copy(session = SessionDisplay.CLOSED)
            .hasAuthenticatedCapability("files.transfer", PresenceDisplay.ONLINE))

        assertFalse(active.copy(presence = RuntimePresence.RECONNECTING)
            .hasAuthenticatedCapability("files.transfer"))
        assertFalse(active.copy(trust = RuntimeTrust.REVOKED)
            .hasAuthenticatedCapability("files.transfer"))
        assertFalse(active.copy(security = RuntimeSecurity.TEST_ONLY)
            .hasAuthenticatedCapability("files.transfer"))
        assertFalse(active.copy(connectivity = RuntimeConnectivity.DISCONNECTED)
            .hasAuthenticatedCapability("files.transfer"))
    }

    @Test
    fun savedTrustedInventoryDoesNotInventActivePresence() {
        val trusted = "a".repeat(64)
        val state = DevicesState.from(
            RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.DISCOVERING),
            listOf(trusted),
        )

        assertNull(state.current)
        assertEquals("a".repeat(16), state.savedPeers.single().shortId)
        assertFalse(state.savedPeers.single().inAuthenticatedSession)
    }

    @Test
    fun savedPeerRequiresExactAuthenticatedFullDeviceId() {
        val activeId = "a".repeat(63) + "1"
        val similarId = "a".repeat(63) + "2"
        val active = RuntimeSnapshot.disconnected().copy(
            peerDeviceId = activeId,
            trust = RuntimeTrust.TRUSTED,
            session = RuntimeSession.ACTIVE,
            connectivity = RuntimeConnectivity.CONNECTED,
            security = RuntimeSecurity.AUTHENTICATED,
        )
        val state = DevicesState.from(active, listOf(similarId, activeId))
        assertFalse(state.savedPeers[0].inAuthenticatedSession)
        assertTrue(state.savedPeers[1].inAuthenticatedSession)

        val offline = DevicesState.from(
            active.copy(connectivity = RuntimeConnectivity.DISCONNECTED),
            listOf(activeId),
        )
        assertFalse(offline.savedPeers.single().inAuthenticatedSession)
    }

    @Test
    fun savedInventoryIsDeduplicatedAndBounded() {
        val ids = (0..25).map { it.toString().padStart(64, '0') }
        val state = DevicesState.from(RuntimeSnapshot.disconnected(), ids + ids.first())

        assertEquals(24, state.savedPeers.size)
        assertEquals(2, state.additionalSavedPeers)
        assertEquals(ids.first().take(16), state.savedPeers.first().shortId)
    }

    @Test
    fun reconnectControlIsOfferedForFailureOrPauseButNeverForRevokedPeer() {
        assertTrue(DevicesState(null, PresenceDisplay.FAILED).offersReconnect())
        assertTrue(DevicesState(null, PresenceDisplay.PAUSED).offersReconnect())
        assertTrue(DevicesState(null, PresenceDisplay.UNAVAILABLE).offersReconnect())
        assertFalse(DevicesState(null, PresenceDisplay.ONLINE).offersReconnect())

        val known = DevicesState.from(
            RuntimeSnapshot.disconnected().copy(
                presence = RuntimePresence.FAILED,
                peerDeviceId = "trusted-peer",
                trust = RuntimeTrust.TRUSTED,
            ),
        )
        assertTrue(known.offersReconnect())
        assertFalse(known.copy(presence = PresenceDisplay.ONLINE).offersReconnect())

        val revoked = DevicesState.from(
            RuntimeSnapshot.disconnected().copy(
                presence = RuntimePresence.FAILED,
                peerDeviceId = "revoked-peer",
                trust = RuntimeTrust.REVOKED,
            ),
        )
        assertFalse(revoked.offersReconnect())
    }

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
