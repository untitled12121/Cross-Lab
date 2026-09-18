package dev.crosslab.android.features.devices

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
    }
}
