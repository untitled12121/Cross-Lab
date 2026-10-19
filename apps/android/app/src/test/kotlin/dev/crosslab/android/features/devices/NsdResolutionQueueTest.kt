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
        val first = requireNotNull(queue.discovered("a", "first", 0))
        assertEquals("first", first.value)
        assertNull(queue.discovered("a", "duplicate", 0))
        assertNull(queue.discovered("b", "second", 0))
        assertNull(queue.discovered("c", "third", 0))
        assertNull(queue.discovered("d", "excess", 0))

        val a = queue.finished(first)
        assertTrue(a.accepted)
        assertEquals("second", a.next?.value)
        val second = requireNotNull(a.next)
        val third = requireNotNull(queue.finished(second).next)
        assertEquals("third", third.value)
        assertNull(queue.finished(third).next)
    }

    @Test
    fun lostServiceCannotBecomeAnAuthenticatedRouteFromLateResolution() {
        val queue = NsdResolutionQueue<String>(3)
        val old = requireNotNull(queue.discovered("a", "old", 0))
        queue.lost("a")
        assertNull(queue.discovered("a", "new", 0))
        val stale = queue.finished(old)
        assertFalse(stale.accepted)
        assertEquals("new", stale.next?.value)
        assertTrue(queue.finished(requireNotNull(stale.next)).accepted)
    }

    @Test
    fun lostQueuedCandidateIsNeverResolved() {
        val queue = NsdResolutionQueue<String>(3)
        val first = requireNotNull(queue.discovered("a", "first", 0))
        queue.discovered("b", "second", 0)
        queue.lost("b")
        assertNull(queue.finished(first).next)
    }

    @Test
    fun duplicateCallbackForReusedNameCannotCompleteNewRequest() {
        val queue = NsdResolutionQueue<String>(2)
        val first = requireNotNull(queue.discovered("same", "old service", 0))
        queue.lost("same")
        queue.discovered("same", "new service", 0)
        val stale = queue.finished(first)
        assertFalse(stale.accepted)
        val second = requireNotNull(stale.next)
        assertEquals("new service", second.value)

        // Android may deliver a late, duplicate callback from the old listener.
        val duplicate = queue.finished(first)
        assertFalse(duplicate.accepted)
        assertNull(duplicate.next)
        assertFalse(queue.finished(first.copy(value = "new service")).accepted)
        assertTrue(queue.finished(second).accepted)
    }

    @Test
    fun shutdownRejectsLateResultsAndNewWork() {
        val queue = NsdResolutionQueue<String>(3)
        val first = requireNotNull(queue.discovered("a", "first", 0))
        queue.close()
        assertFalse(queue.finished(first).accepted)
        assertNull(queue.discovered("b", "second", 0))
    }
}
