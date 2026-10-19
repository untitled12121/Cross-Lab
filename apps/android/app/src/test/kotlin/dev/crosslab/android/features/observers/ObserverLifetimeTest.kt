package dev.crosslab.android.features.observers

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ObserverLifetimeTest {
    @Test
    fun queuedUpdatesAreRejectedAfterOwnerTeardown() {
        val tasks = mutableListOf<() -> Unit>()
        val gate = UiCallbackGate { tasks += it }
        var called = 0
        gate.dispatch { called++ }
        assertEquals(1, tasks.size)
        gate.close()
        tasks.single().invoke()
        gate.dispatch { called++ }
        assertEquals(0, called)
        assertEquals(1, tasks.size)
    }

    @Test
    fun healthyCallbacksRemainOrderedWhenOwnerIsAlive() {
        val tasks = mutableListOf<() -> Unit>()
        val gate = UiCallbackGate { tasks += it }
        val delivered = mutableListOf<Int>()
        gate.dispatch { delivered += 1 }
        gate.dispatch { delivered += 2 }
        tasks.forEach { it() }
        assertEquals(listOf(1, 2), delivered)
    }

    @Test
    fun brokenObserverUpdateDoesNotPreventLaterUiDelivery() {
        val tasks = mutableListOf<() -> Unit>()
        val gate = UiCallbackGate { tasks += it }
        var reached = false
        gate.dispatch { throw IllegalStateException("view disposed") }
        gate.dispatch { reached = true }
        tasks.forEach { it() }
        assertTrue(reached)
    }

    @Test
    fun dispatcherFailureDoesNotEscapeNativeCallbacks() {
        val gate = UiCallbackGate { throw IllegalStateException("handler stopped") }
        var executed = false
        gate.dispatch { executed = true }
        assertFalse(executed)
        gate.close()
        gate.dispatch { executed = true }
        assertFalse(executed)
    }

    @Test
    fun failedSubscriptionCloseNeverSkipsRemainingCleanup() {
        val closed = mutableListOf<Int>()
        closeObservers(
            listOf(
                AutoCloseable { closed += 1; error("broken close") },
                null,
                AutoCloseable { closed += 2 },
                AutoCloseable { closed += 3 },
            ),
        )
        assertEquals(listOf(1, 2, 3), closed)
    }

    @Test
    fun closeCanBeRepeatedWithoutReenablingQueuedUiEvents() {
        val tasks = mutableListOf<() -> Unit>()
        val gate = UiCallbackGate { tasks += it }
        var called = false
        gate.dispatch { called = true }
        gate.close()
        gate.close()
        tasks.forEach { it() }
        assertFalse(called)
    }
}
