package dev.crosslab.android.features.filetransfer

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TransferPublicationGateTest {
    @Test
    fun cancellation_before_publication_prevents_save() {
        val gate = TransferPublicationGate()
        assertTrue(gate.cancel())
        assertTrue(gate.cancelled())
        assertFalse(gate.beginPublication())
    }

    @Test
    fun concurrent_cancellation_and_publication_have_exactly_one_winner() {
        repeat(64) {
            val gate = TransferPublicationGate()
            val start = CountDownLatch(1)
            val finished = CountDownLatch(2)
            val cancelled = AtomicBoolean(false)
            val published = AtomicBoolean(false)
            val threads = listOf(
                Thread {
                    start.await()
                    cancelled.set(gate.cancel())
                    finished.countDown()
                },
                Thread {
                    start.await()
                    published.set(gate.beginPublication())
                    finished.countDown()
                },
            )
            threads.forEach(Thread::start)
            start.countDown()
            assertTrue(finished.await(5, TimeUnit.SECONDS))
            threads.forEach(Thread::join)
            assertTrue(cancelled.get() != published.get())
            assertEquals(cancelled.get(), gate.cancelled())
        }
    }

    @Test
    fun publication_commit_rejects_late_cancellation() {
        val gate = TransferPublicationGate()
        assertTrue(gate.beginPublication())
        assertFalse(gate.cancel())
        assertFalse(gate.cancelled())
        assertFalse(gate.beginPublication())
    }
}
