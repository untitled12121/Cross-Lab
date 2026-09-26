package dev.crosslab.android.features.devices

import java.util.concurrent.CopyOnWriteArraySet
import org.junit.Assert.assertEquals
import org.junit.Test

class RuntimeControllerTest {
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
        var stops = 0
        var networkLost = 0
        var networkAvailable = 0
        var disconnects = 0
        var reconnects = 0
        var permissionEdits = 0
        private var current = RuntimeSnapshot.disconnected()
        private val snapshotListeners = CopyOnWriteArraySet<(RuntimeSnapshot) -> Unit>()

        override fun start() {
            starts += 1
        }

        override fun stop() {
            stops += 1
        }

        override fun networkLost() {
            networkLost += 1
        }

        override fun networkAvailable() {
            networkAvailable += 1
        }

        override fun disconnectPeer() {
            disconnects += 1
        }

        override fun reconnectPeer() {
            reconnects += 1
        }

        override fun setPermission(
            capabilityId: String,
            operation: String,
            effect: RuntimePermissionEffect,
        ): Boolean {
            permissionEdits += 1
            return true
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
