package dev.crosslab.android.features.notifications

import dev.crosslab.android.features.devices.RuntimeConnectivity
import dev.crosslab.android.features.devices.RuntimePresence
import dev.crosslab.android.features.devices.RuntimeSecurity
import dev.crosslab.android.features.devices.RuntimeSession
import dev.crosslab.android.features.devices.RuntimeSnapshot
import dev.crosslab.android.features.devices.RuntimeTrust
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NotificationSessionGateTest {
    @Test
    fun capture_requires_fresh_authenticated_negotiated_notification_session() {
        val active =
            RuntimeSnapshot.disconnected().copy(
                presence = RuntimePresence.ONLINE,
                peerDeviceId = "authenticated-test-peer",
                trust = RuntimeTrust.TRUSTED,
                connectivity = RuntimeConnectivity.CONNECTED,
                session = RuntimeSession.ACTIVE,
                security = RuntimeSecurity.AUTHENTICATED,
                capabilityIds = listOf("notifications.read"),
            )
        assertTrue(active.canMirrorNotifications())
        assertFalse(active.copy(presence = RuntimePresence.RECONNECTING).canMirrorNotifications())
        assertFalse(active.copy(peerDeviceId = null).canMirrorNotifications())
        assertFalse(active.copy(trust = RuntimeTrust.REVOKED).canMirrorNotifications())
        assertFalse(active.copy(connectivity = RuntimeConnectivity.DISCONNECTED).canMirrorNotifications())
        assertFalse(active.copy(session = RuntimeSession.CLOSED).canMirrorNotifications())
        assertFalse(active.copy(security = RuntimeSecurity.TEST_ONLY).canMirrorNotifications())
        assertFalse(active.copy(capabilityIds = emptyList()).canMirrorNotifications())
    }
}
