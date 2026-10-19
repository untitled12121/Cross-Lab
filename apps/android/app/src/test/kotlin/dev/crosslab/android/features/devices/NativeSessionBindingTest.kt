package dev.crosslab.android.features.devices

import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NativeSessionBindingTest {
    private val trusted =
        RuntimeSnapshot.disconnected().copy(
            presence = RuntimePresence.ONLINE,
            peerDeviceId = "trusted-peer",
            trust = RuntimeTrust.TRUSTED,
            connectivity = RuntimeConnectivity.CONNECTED,
            session = RuntimeSession.ACTIVE,
            security = RuntimeSecurity.AUTHENTICATED,
            capabilityIds = listOf("clipboard.read"),
        )

    @Test
    fun stalePeerOrSessionCannotReuseClipboardHandoffToken() {
        val first = "ab".repeat(32)
        val token = authenticatedSessionBinding(trusted, first)
        assertNotNull(token)
        assertTrue(checkNotNull(token).matches(authenticatedSessionBinding(trusted, first)))
        assertFalse(token.matches(authenticatedSessionBinding(trusted, "cd".repeat(32))))
        assertFalse(token.matches(authenticatedSessionBinding(
            trusted.copy(peerDeviceId = "different-trusted-peer"), first,
        )))
        assertFalse(token.toString().contains(first))
    }

    @Test
    fun offlineRevokedAndUnauthenticatedSnapshotsCannotMintTokens() {
        val id = "ab".repeat(32)
        assertNull(authenticatedSessionBinding(trusted, null))
        assertNull(authenticatedSessionBinding(trusted, "invalid"))
        assertNull(authenticatedSessionBinding(trusted, "ZZ".repeat(32)))
        assertNull(authenticatedSessionBinding(trusted.copy(presence = RuntimePresence.RECONNECTING), id))
        assertNull(authenticatedSessionBinding(trusted.copy(trust = RuntimeTrust.REVOKED), id))
        assertNull(authenticatedSessionBinding(trusted.copy(session = RuntimeSession.CLOSED), id))
        assertNull(authenticatedSessionBinding(trusted.copy(connectivity = RuntimeConnectivity.DISCONNECTED), id))
        assertNull(authenticatedSessionBinding(trusted.copy(security = RuntimeSecurity.TEST_ONLY), id))
    }
}
