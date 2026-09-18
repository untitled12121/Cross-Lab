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
    fun connectedTrustedSnapshotMapsToPresentationState() {
        val state = DevicesState.from(
            RuntimeSnapshot(
                peerDeviceId = "0123456789abcdef".repeat(4),
                trust = RuntimeTrust.TRUSTED,
                connectivity = RuntimeConnectivity.CONNECTED,
                session = RuntimeSession.ACTIVE,
                protocol = RuntimeProtocolVersion(1, 4),
                network = RuntimeNetwork.LOCAL,
                security = RuntimeSecurity.AUTHENTICATED,
                metered = false,
                capabilityCount = 2,
            ),
        )

        val device = checkNotNull(state.current)
        assertEquals("0123456789abcdef", device.peerId)
        assertEquals(TrustDisplay.TRUSTED, device.trust)
        assertEquals(ConnectivityDisplay.CONNECTED, device.connectivity)
        assertEquals(SessionDisplay.ACTIVE, device.session)
        assertEquals("1.4", device.protocol)
        assertEquals(NetworkDisplay.LOCAL, device.network)
        assertEquals(SecurityDisplay.AUTHENTICATED, device.security)
        assertEquals(2, device.capabilityCount)
    }

    @Test
    fun revokedSnapshotRemainsRevokedAndDisconnected() {
        val state = DevicesState.from(
            RuntimeSnapshot(
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
