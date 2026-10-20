package dev.crosslab.android.features.devices

import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NativeListenerLoopTest {
    @Test
    fun unexpectedNativeWaitFailureInvalidatesCurrentListener() {
        val current = AtomicBoolean(true)
        var faults = 0
        val result = runCatching {
            runNativeListenerLoop(
                isCurrent = current::get,
                waitNext = { throw IllegalStateException("native event source closed") },
                deliver = { _: String -> error("no event may be delivered") },
                unexpectedExit = {
                    faults++
                    current.set(false)
                },
            )
        }
        assertTrue(result.isFailure)
        assertEquals(1, faults)
        assertFalse(current.get())
    }

    @Test
    fun nativeHandlerFailureAlsoFailsClosed() {
        val current = AtomicBoolean(true)
        var faults = 0
        val result = runCatching {
            runNativeListenerLoop(
                isCurrent = current::get,
                waitNext = { "authenticated event" },
                deliver = { _: String -> error("simulated observer failure") },
                unexpectedExit = {
                    faults++
                    current.set(false)
                },
            )
        }
        assertTrue(result.isFailure)
        assertEquals(1, faults)
    }

    @Test
    fun staleGenerationCannotDeliverOrFaultAReplacementSession() {
        val generation = AtomicInteger(1)
        var delivered = 0
        var faults = 0
        runNativeListenerLoop(
            isCurrent = { generation.get() == 1 },
            waitNext = {
                generation.set(2)
                "stale callback"
            },
            deliver = { _: String -> delivered++ },
            unexpectedExit = { faults++ },
        )
        assertEquals(0, delivered)
        assertEquals(0, faults)
    }

    @Test
    fun timeoutsAreNotFaultsAndIntentionalStopIsQuiet() {
        val current = AtomicBoolean(true)
        var polls = 0
        var faults = 0
        runNativeListenerLoop(
            isCurrent = current::get,
            waitNext = {
                polls++
                if (polls == 3) current.set(false)
                null as String?
            },
            deliver = { _: String -> error("timeout has no payload") },
            unexpectedExit = { faults++ },
        )
        assertEquals(3, polls)
        assertEquals(0, faults)
    }

    @Test
    fun oldNativeErrorAfterGenerationChangeIsNotAFalseDisconnect() {
        val generation = AtomicInteger(1)
        var faults = 0
        val result = runCatching {
            runNativeListenerLoop(
                isCurrent = { generation.get() == 1 },
                waitNext = {
                    generation.set(2)
                    throw IllegalStateException("old listener was closed")
                },
                deliver = { _: String -> error("no stale event") },
                unexpectedExit = { faults++ },
            )
        }
        assertTrue(result.isFailure)
        assertEquals(0, faults)
    }
}
