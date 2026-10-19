package dev.crosslab.android.features.devices

import java.util.concurrent.CopyOnWriteArraySet
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class RuntimeControllerTest {
    @Test
    fun filePickerTicketRequiresLiveAuthenticatedNegotiatedSession() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        val token = NativeSessionBinding("trusted-peer", "ab".repeat(32))
        port.binding = token
        val online = RuntimeSnapshot.disconnected().copy(
            presence = RuntimePresence.ONLINE,
            peerDeviceId = "trusted-peer",
            trust = RuntimeTrust.TRUSTED,
            connectivity = RuntimeConnectivity.CONNECTED,
            session = RuntimeSession.ACTIVE,
            security = RuntimeSecurity.AUTHENTICATED,
            capabilityIds = listOf("files.transfer"),
        )

        port.emit(online)
        assertNull(controller.sessionBinding("files.transfer"))
        controller.onForeground()
        assertTrue(token.matches(controller.sessionBinding("files.transfer")))
        assertNull(controller.sessionBinding("clipboard.read"))

        controller.onNetworkLost()
        assertNull(controller.sessionBinding("files.transfer"))
        controller.onNetworkAvailable()
        port.emit(online.copy(presence = RuntimePresence.RECONNECTING))
        assertNull(controller.sessionBinding("files.transfer"))
        port.emit(online.copy(trust = RuntimeTrust.REVOKED))
        assertNull(controller.sessionBinding("files.transfer"))
        port.emit(online)
        controller.shutdown()
        assertNull(controller.sessionBinding("files.transfer"))
    }

    @Test
    fun networkCallbackExceptionFailsClosedBeforeAnyMoreNativeActions() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        controller.onForeground()
        port.failNextNetworkLost = true

        controller.onNetworkLost()

        assertEquals(RuntimeLifecycle.STOPPED, controller.state().lifecycle)
        assertEquals(false, controller.state().networkAvailable)
        assertEquals(RuntimePresence.FAILED, controller.state().snapshot.presence)
        assertEquals(1, port.stops)
        assertNull(controller.sessionBinding("files.transfer"))

        controller.onNetworkAvailable()
        controller.reconnectPeer()
        assertEquals(RuntimeLifecycle.RUNNING, controller.state().lifecycle)
        assertEquals(2, port.starts)
    }

    @Test
    fun failedReconnectDoesNotPreserveStaleAuthenticatedSnapshot() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        controller.onForeground()
        port.emit(RuntimeSnapshot.disconnected().copy(
            presence = RuntimePresence.ONLINE,
            peerDeviceId = "trusted-peer",
            trust = RuntimeTrust.TRUSTED,
            connectivity = RuntimeConnectivity.CONNECTED,
            session = RuntimeSession.ACTIVE,
            security = RuntimeSecurity.AUTHENTICATED,
            capabilityIds = listOf("files.transfer"),
        ))
        port.failNextReconnect = true
        controller.reconnectPeer()

        assertEquals(RuntimeLifecycle.STOPPED, controller.state().lifecycle)
        assertEquals(RuntimePresence.FAILED, controller.state().snapshot.presence)
        assertNull(controller.state().snapshot.peerDeviceId)
    }

    @Test
    fun failedBackgroundStopTerminatesTheNativeRuntime() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        controller.onForeground()
        port.failNextStop = true
        controller.onBackground()

        assertEquals(RuntimeLifecycle.SHUTDOWN, controller.state().lifecycle)
        assertEquals(1, port.shutdowns)
        controller.reconnectPeer()
        assertEquals(1, port.starts)
    }

    @Test
    fun foregroundAndBackgroundAreIdempotent() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)

        controller.onForeground()
        controller.onForeground()
        controller.onBackground()
        controller.onBackground()

        assertEquals(1, port.starts)
        assertEquals(1, port.stops)
        assertEquals(RuntimeLifecycle.STOPPED, controller.state().lifecycle)
    }

    @Test
    fun stoppedRuntimeIgnoresLateAuthenticatedSnapshotCallbacks() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        controller.onForeground()
        controller.onBackground()
        port.emit(RuntimeSnapshot.disconnected().copy(
            presence = RuntimePresence.ONLINE,
            peerDeviceId = "stale-peer",
            trust = RuntimeTrust.TRUSTED,
            connectivity = RuntimeConnectivity.CONNECTED,
            session = RuntimeSession.ACTIVE,
            security = RuntimeSecurity.AUTHENTICATED,
            capabilityIds = listOf("files.transfer"),
        ))
        assertEquals(RuntimeLifecycle.STOPPED, controller.state().lifecycle)
        assertEquals(RuntimeSnapshot.disconnected(), controller.state().snapshot)
        assertNull(controller.sessionBinding("files.transfer"))
    }

    @Test
    fun networkCommandsOnlyReachARunningRuntime() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)

        controller.onNetworkLost()
        controller.onNetworkAvailable()
        controller.onForeground()
        controller.onNetworkLost()
        controller.onNetworkAvailable()
        controller.onBackground()
        controller.onNetworkLost()

        assertEquals(1, port.networkLost)
        assertEquals(1, port.networkAvailable)
        assertEquals(false, controller.state().networkAvailable)
    }

    @Test
    fun networkChangesWhileStoppedAreReappliedBeforeSessionsResume() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)

        controller.onNetworkLost()
        assertEquals(false, controller.state().networkAvailable)
        assertEquals(0, port.networkLost)

        controller.onForeground()
        assertEquals(1, port.starts)
        assertEquals(1, port.networkLost)
        assertEquals(false, controller.state().networkAvailable)

        controller.reloadTrust()
        assertEquals(2, port.starts)
        assertEquals(2, port.networkLost)

        controller.onNetworkAvailable()
        assertEquals(true, controller.state().networkAvailable)
        assertEquals(1, port.networkAvailable)
    }

    @Test
    fun repeatedNetworkCallbacksDoNotRestartDiscovery() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)

        controller.onForeground()
        controller.onNetworkAvailable()
        controller.onNetworkLost()
        controller.onNetworkLost()
        controller.onNetworkAvailable()
        controller.onNetworkAvailable()

        assertEquals(1, port.networkLost)
        assertEquals(1, port.networkAvailable)
        assertEquals(true, controller.state().networkAvailable)

        controller.shutdown()
        controller.onNetworkLost()
        assertEquals(1, port.networkLost)
        assertEquals(true, controller.state().networkAvailable)
    }

    @Test
    fun nativeSnapshotUpdatesReachPresentationState() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        val connected =
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
                capabilityCount = 2,
            )

        controller.onForeground()
        port.emit(connected)

        assertEquals(connected, controller.state().snapshot)
    }

    @Test
    fun userPeerControlsOnlyReachARunningRuntime() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)

        controller.disconnectPeer()
        controller.reconnectPeer()
        controller.onForeground()
        controller.disconnectPeer()
        controller.reconnectPeer()
        controller.onBackground()
        controller.disconnectPeer()

        assertEquals(1, port.disconnects)
        assertEquals(1, port.reconnects)
    }

    @Test
    fun permissionEditsOnlyReachARunningRuntime() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)

        assertEquals(
            false,
            controller.setPermission(
                "clipboard.read",
                "get",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        controller.onForeground()
        assertEquals(
            false,
            controller.setPermission(
                "clipboard.read",
                "get",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        port.emit(
            RuntimeSnapshot.disconnected().copy(
                peerDeviceId = "trusted-peer",
                trust = RuntimeTrust.TRUSTED,
                connectivity = RuntimeConnectivity.CONNECTED,
                session = RuntimeSession.ACTIVE,
                security = RuntimeSecurity.AUTHENTICATED,
                capabilityIds = listOf("clipboard.read"),
            ),
        )
        assertEquals(
            true,
            controller.setPermission(
                "clipboard.read",
                "get",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        controller.onBackground()
        controller.setPermission(
            "clipboard.read",
            "get",
            RuntimePermissionEffect.DENY,
        )

        assertEquals(1, port.permissionEdits)
    }

    @Test
    fun peerBoundPermissionNeverAppliesAfterSessionSwitch() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        val first =
            RuntimeSnapshot.disconnected().copy(
                peerDeviceId = "first-trusted-id",
                session = RuntimeSession.ACTIVE,
                trust = RuntimeTrust.TRUSTED,
                connectivity = RuntimeConnectivity.CONNECTED,
                security = RuntimeSecurity.AUTHENTICATED,
                capabilityIds = listOf("notifications.read"),
            )
        val second = first.copy(peerDeviceId = "different-trusted-id")
        port.emit(first)
        controller.onForeground()
        assertEquals(
            false,
            controller.setPermissionForPeer(
                "different-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        assertEquals(
            true,
            controller.setPermissionForPeer(
                "first-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        port.emit(second)
        assertEquals(
            false,
            controller.setPermissionForPeer(
                "first-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        assertEquals(1, port.peerPermissionEdits)
        port.emit(second.copy(trust = RuntimeTrust.REVOKED))
        assertEquals(
            false,
            controller.setPermissionForPeer(
                "different-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        port.emit(second.copy(security = RuntimeSecurity.TEST_ONLY))
        assertEquals(
            false,
            controller.setPermissionForPeer(
                "different-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        port.emit(second.copy(capabilityIds = emptyList()))
        assertEquals(
            false,
            controller.setPermissionForPeer(
                "different-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        controller.onNetworkLost()
        port.emit(second)
        assertEquals(
            false,
            controller.setPermissionForPeer(
                "different-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
        assertEquals(1, port.peerPermissionEdits)
        controller.onBackground()
        assertEquals(
            false,
            controller.setPermissionForPeer(
                "different-trusted-id",
                "notifications.read",
                "subscribe",
                RuntimePermissionEffect.ALLOW,
            ),
        )
    }

    @Test
    fun trustReloadRestartsOnlyAfterClosingPriorRuntime() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        controller.onForeground()
        assertEquals(true, controller.reloadTrust())
        assertEquals(2, port.starts)
        assertEquals(1, port.stops)
        assertEquals(RuntimeLifecycle.RUNNING, controller.state().lifecycle)
    }

    @Test
    fun trustReloadFailureRemainsStopped() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        controller.onForeground()
        port.failNextStart = true
        assertEquals(false, controller.reloadTrust())
        assertEquals(RuntimeLifecycle.STOPPED, controller.state().lifecycle)
        assertEquals(2, port.stops)
    }

    @Test
    fun failedForegroundStartLeavesRuntimeStoppedAndRetryCanRecover() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        port.failNextStart = true

        controller.onForeground()

        assertEquals(RuntimeLifecycle.STOPPED, controller.state().lifecycle)
        assertEquals(RuntimePresence.FAILED, controller.state().snapshot.presence)
        assertNull(controller.state().snapshot.peerDeviceId)
        assertEquals(1, port.starts)
        assertEquals(1, port.stops)

        controller.reconnectPeer()

        assertEquals(RuntimeLifecycle.RUNNING, controller.state().lifecycle)
        assertEquals(2, port.starts)
        assertEquals(0, port.reconnects)
    }

    @Test
    fun retryAfterFailedOfflineStartPreservesNetworkState() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)
        controller.onNetworkLost()
        port.failNextStart = true

        controller.onForeground()
        assertEquals(RuntimeLifecycle.STOPPED, controller.state().lifecycle)
        assertEquals(false, controller.state().networkAvailable)

        controller.reconnectPeer()

        assertEquals(RuntimeLifecycle.RUNNING, controller.state().lifecycle)
        assertEquals(false, controller.state().networkAvailable)
        assertEquals(1, port.networkLost)
    }

    @Test
    fun shutdownStopsOnceAndPreventsRestart() {
        val port = FakeRuntimePort()
        val controller = RuntimeController(port)

        controller.onForeground()
        controller.shutdown()
        controller.shutdown()
        controller.onForeground()

        assertEquals(1, port.starts)
        assertEquals(1, port.stops)
        assertEquals(RuntimeLifecycle.SHUTDOWN, controller.state().lifecycle)
    }

    private class FakeRuntimePort : RuntimePort {
        var starts = 0
        var failNextStart = false
        var failNextStop = false
        var failNextNetworkLost = false
        var failNextReconnect = false
        var stops = 0
        var shutdowns = 0
        var networkLost = 0
        var networkAvailable = 0
        var disconnects = 0
        var reconnects = 0
        var permissionEdits = 0
        var peerPermissionEdits = 0
        var binding: NativeSessionBinding? = null
        private var current = RuntimeSnapshot.disconnected()
        private val snapshotListeners = CopyOnWriteArraySet<(RuntimeSnapshot) -> Unit>()

        override fun start() {
            starts += 1
            if (failNextStart) {
                failNextStart = false
                throw IllegalStateException("simulated secure runtime start failure")
            }
        }

        override fun stop() {
            stops += 1
            if (failNextStop) {
                failNextStop = false
                throw IllegalStateException("simulated stop failure")
            }
        }

        override fun networkLost() {
            networkLost += 1
            if (failNextNetworkLost) {
                failNextNetworkLost = false
                throw IllegalStateException("simulated network-loss callback failure")
            }
        }

        override fun networkAvailable() {
            networkAvailable += 1
        }

        override fun disconnectPeer() {
            disconnects += 1
        }

        override fun reconnectPeer() {
            reconnects += 1
            if (failNextReconnect) {
                failNextReconnect = false
                throw IllegalStateException("simulated reconnect failure")
            }
        }

        override fun setPermission(
            capabilityId: String,
            operation: String,
            effect: RuntimePermissionEffect,
        ): Boolean {
            permissionEdits += 1
            return true
        }

        override fun setPermissionForPeer(
            peerDeviceId: String,
            capabilityId: String,
            operation: String,
            effect: RuntimePermissionEffect,
        ): Boolean {
            if (current.peerDeviceId != peerDeviceId || current.session != RuntimeSession.ACTIVE) {
                return false
            }
            peerPermissionEdits += 1
            return true
        }

        override fun sessionBinding(capabilityId: String): NativeSessionBinding? = binding

        override fun shutdown() {
            shutdowns += 1
        }

        override fun snapshot(): RuntimeSnapshot = current

        override fun observeSnapshots(listener: (RuntimeSnapshot) -> Unit): AutoCloseable {
            snapshotListeners += listener
            listener(current)
            return AutoCloseable { snapshotListeners -= listener }
        }

        fun emit(snapshot: RuntimeSnapshot) {
            current = snapshot
            snapshotListeners.forEach { it(snapshot) }
        }
    }
}
