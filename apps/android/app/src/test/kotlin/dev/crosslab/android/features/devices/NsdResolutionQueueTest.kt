package dev.crosslab.android.features.devices

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NsdResolutionQueueTest {
    @Test
    fun resolvesSeriallyAndBoundsPendingServices() {
        val queue = NsdResolutionQueue<String>(3)
        assertEquals("first", queue.discovered("a", "first", 0)?.value)
        assertNull(queue.discovered("a", "duplicate", 0))
        assertNull(queue.discovered("b", "second", 0))
        assertNull(queue.discovered("c", "third", 0))
        assertNull(queue.discovered("d", "excess", 0))

        val a = queue.finished("a")
        assertTrue(a.accepted)
        assertEquals("second", a.next?.value)
        assertEquals("third", queue.finished("b").next?.value)
        assertNull(queue.finished("c").next)
    }

    @Test
    fun lostServiceCannotBecomeAnAuthenticatedRouteFromLateResolution() {
        val queue = NsdResolutionQueue<String>(3)
        queue.discovered("a", "old", 0)
        queue.lost("a")
        assertNull(queue.discovered("a", "new", 0))
        val stale = queue.finished("a")
        assertFalse(stale.accepted)
        assertEquals("new", stale.next?.value)
        assertTrue(queue.finished("a").accepted)
    }

    @Test
    fun lostQueuedCandidateIsNeverResolved() {
        val queue = NsdResolutionQueue<String>(3)
        queue.discovered("a", "first", 0)
        queue.discovered("b", "second", 0)
        queue.lost("b")
        assertNull(queue.finished("a").next)
    }

    @Test
    fun shutdownRejectsLateResultsAndNewWork() {
        val queue = NsdResolutionQueue<String>(3)
        queue.discovered("a", "first", 0)
        queue.close()
        assertFalse(queue.finished("a").accepted)
        assertNull(queue.discovered("b", "second", 0))
    }
}
