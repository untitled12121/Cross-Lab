package dev.crosslab.android.features.observers

import java.util.concurrent.CopyOnWriteArraySet
import org.junit.Assert.assertEquals
import org.junit.Test

class ObserverDeliveryTest {
    @Test
    fun failingObserverCannotBlockLaterSubscribersOrStateChanges() {
        val listeners = CopyOnWriteArraySet<(Int) -> Unit>()
        val updates = mutableListOf<Int>()
        listeners += { throw IllegalStateException("UI disconnected") }
        listeners += { value -> updates += value }

        notifyObservers(listeners, 1)
        notifyObservers(listeners, 2)

        assertEquals(listOf(1, 2), updates)
        assertEquals(2, listeners.size)
    }
}
