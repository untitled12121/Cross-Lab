package dev.crosslab.android.features.audit

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlin.concurrent.thread
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class OwnerAuditActionGateTest {
    @Test fun parallelOwnerOperationsCannotOverlap() {
        val gate = OwnerAuditActionGate()
        val first = checkNotNull(gate.begin(OwnerAuditAction.REFRESH))
        assertTrue(gate.isBusy())
        assertNull(gate.begin(OwnerAuditAction.CLEAR))
        assertNull(gate.begin(OwnerAuditAction.EXPORT))
        assertTrue(gate.finish(first))
        assertFalse(gate.isBusy())
        val next = checkNotNull(gate.begin(OwnerAuditAction.CLEAR))
        assertTrue(next > first)
    }

    @Test fun duplicateOrStaleCallbackNeverCompletesNewWork() {
        val gate = OwnerAuditActionGate()
        val first = checkNotNull(gate.begin(OwnerAuditAction.EXPORT))
        assertTrue(gate.finish(first))
        val next = checkNotNull(gate.begin(OwnerAuditAction.REFRESH))
        assertFalse(gate.finish(first))
        assertTrue(gate.isBusy())
        assertTrue(gate.finish(next))
        assertFalse(gate.finish(next))
    }

    @Test fun cancelledFilePickerReleasesItsTicket() {
        val gate = OwnerAuditActionGate()
        val pending = checkNotNull(gate.begin(OwnerAuditAction.EXPORT))
        assertTrue(gate.finish(pending))
        assertNotNull(gate.begin(OwnerAuditAction.CLEAR))
    }

    @Test fun shutdownRejectsQueuedLateCallbacksAndNewCommands() {
        val gate = OwnerAuditActionGate()
        val pending = checkNotNull(gate.begin(OwnerAuditAction.CLEAR))
        gate.close()
        gate.close()
        assertFalse(gate.finish(pending))
        assertFalse(gate.isBusy())
        assertNull(gate.begin(OwnerAuditAction.EXPORT))
    }

    @Test fun concurrentOwnerRequestsAdmitOnlyOne() {
        val gate = OwnerAuditActionGate()
        val go = CountDownLatch(1)
        val ready = CountDownLatch(6)
        val accepted = AtomicInteger()
        val workers = (0 until 6).map {
            thread {
                ready.countDown()
                go.await(5, TimeUnit.SECONDS)
                if (gate.begin(OwnerAuditAction.REFRESH) != null) accepted.incrementAndGet()
            }
        }
        assertTrue(ready.await(5, TimeUnit.SECONDS))
        go.countDown()
        workers.forEach { it.join(5_000) }
        assertEquals(1, accepted.get())
    }
}
