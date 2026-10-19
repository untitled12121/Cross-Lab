package dev.crosslab.android.features.devices

import dev.crosslab.android.components.ui.StatusTone
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RuntimeRecoveryPresentationTest {
    private fun runtime(
        lifecycle: RuntimeLifecycle = RuntimeLifecycle.RUNNING,
        network: Boolean = true,
        canControl: Boolean = true,
        snapshot: RuntimeSnapshot = RuntimeSnapshot.disconnected(),
    ) = RuntimeControllerState(lifecycle, network, canControl, snapshot)

    private fun presentation(state: RuntimeControllerState) =
        RuntimeRecoveryPresentation.from(state, DevicesState.from(state.snapshot))

    private fun active() = RuntimeSnapshot.disconnected().copy(
        presence = RuntimePresence.ONLINE,
        peerDeviceId = "exact-trusted-device",
        trust = RuntimeTrust.TRUSTED,
        connectivity = RuntimeConnectivity.CONNECTED,
        session = RuntimeSession.ACTIVE,
        security = RuntimeSecurity.AUTHENTICATED,
    )

    @Test
    fun authenticatedSessionOffersDisconnectOnlyWithExactSecurityGates() {
        val valid = presentation(runtime(snapshot = active()))
        assertEquals("Authenticated session", valid.label)
        assertEquals(RuntimeRecoveryAction.DISCONNECT, valid.action)
        assertTrue(valid.actionEnabled)
        assertEquals(StatusTone.ACCENT, valid.tone)

        val variants = listOf(
            active().copy(peerDeviceId = null),
            active().copy(trust = RuntimeTrust.REVOKED),
            active().copy(connectivity = RuntimeConnectivity.DISCONNECTED),
            active().copy(session = RuntimeSession.CLOSED),
            active().copy(security = RuntimeSecurity.TEST_ONLY),
        )
        variants.forEach { invalid ->
            val result = presentation(runtime(snapshot = invalid))
            assertEquals(
                if (invalid.trust == RuntimeTrust.REVOKED) "Peer revoked"
                else "Session not verified",
                result.label,
            )
            assertEquals(StatusTone.CRITICAL, result.tone)
            assertEquals(
                if (invalid.trust == RuntimeTrust.REVOKED) RuntimeRecoveryAction.NONE
                else RuntimeRecoveryAction.RECONNECT,
                result.action,
            )
        }
    }

    @Test
    fun aRevokedRuntimeSessionCannotOfferReconnectFromAStaleOnlinePresence() {
        val state = presentation(runtime(snapshot = active().copy(session = RuntimeSession.REVOKED)))
        assertEquals("Peer revoked", state.label)
        assertEquals(RuntimeRecoveryAction.NONE, state.action)
        assertFalse(state.actionEnabled)
    }

    @Test
    fun terminalShutdownNeverOffersRetryOrPeerControl() {
        val result = presentation(
            runtime(RuntimeLifecycle.SHUTDOWN, snapshot = active()),
        )
        assertEquals("Runtime shut down", result.label)
        assertEquals(RuntimeRecoveryAction.NONE, result.action)
        assertFalse(result.actionEnabled)
        assertEquals(StatusTone.CRITICAL, result.tone)
    }

    @Test
    fun stoppedNativeFailureExplainsSafeRetryButNetworkBlocksIt() {
        val failed = RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED)
        val result = presentation(runtime(RuntimeLifecycle.STOPPED, snapshot = failed))
        assertEquals("Runtime failed", result.label)
        assertEquals(RuntimeRecoveryAction.RETRY, result.action)
        assertTrue(result.actionEnabled)
        assertEquals(StatusTone.CRITICAL, result.tone)
        val offline = presentation(
            runtime(RuntimeLifecycle.STOPPED, network = false, snapshot = failed),
        )
        assertEquals(RuntimeRecoveryAction.RETRY, offline.action)
        assertFalse(offline.actionEnabled)
    }

    @Test
    fun networkLossCannotOfferDisconnectEvenWhenOldSessionSnapshotIsOnline() {
        val state = presentation(runtime(network = false, snapshot = active()))
        assertEquals("LAN unavailable", state.label)
        assertEquals(RuntimeRecoveryAction.NONE, state.action)
        assertFalse(state.actionEnabled)
    }

    @Test
    fun reconnectOnlyForPausedFailedOrUnavailablePresence() {
        listOf(RuntimePresence.PAUSED, RuntimePresence.FAILED, RuntimePresence.UNAVAILABLE)
            .forEach { presence ->
                val state = presentation(
                    runtime(snapshot = RuntimeSnapshot.disconnected().copy(presence = presence)),
                )
                assertEquals(RuntimeRecoveryAction.RECONNECT, state.action)
                assertTrue(state.actionEnabled)
            }
        listOf(RuntimePresence.CONNECTING, RuntimePresence.DISCOVERING, RuntimePresence.RECONNECTING)
            .forEach { presence ->
                val state = presentation(
                    runtime(snapshot = RuntimeSnapshot.disconnected().copy(presence = presence)),
                )
                assertEquals(RuntimeRecoveryAction.NONE, state.action)
                assertFalse(state.actionEnabled)
            }
    }

    @Test
    fun unavailableNativePeerControlsNeverBecomeUiActions() {
        val online = presentation(runtime(canControl = false, snapshot = active()))
        assertEquals(RuntimeRecoveryAction.NONE, online.action)
        assertFalse(online.actionEnabled)
        val failed = presentation(
            runtime(
                RuntimeLifecycle.STOPPED,
                canControl = false,
                snapshot = RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
            ),
        )
        assertEquals(RuntimeRecoveryAction.NONE, failed.action)
        assertFalse(failed.actionEnabled)
    }
}
