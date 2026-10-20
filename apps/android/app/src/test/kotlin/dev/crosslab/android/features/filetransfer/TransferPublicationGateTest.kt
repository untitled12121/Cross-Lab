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
    fun teardownBlocksPublicationEvenWhenCleanupQueueIsSaturated() {
        val gate = TransferPublicationGate()
        val pool = BoundedTransferWorkerPool("crosslab-test-receive-cleanup", 1, 1)
        val started = CountDownLatch(1)
        val release = CountDownLatch(1)
        try {
            assertTrue(pool.submit {
                started.countDown()
                release.await(5, TimeUnit.SECONDS)
            })
            assertTrue(started.await(5, TimeUnit.SECONDS))
            assertTrue(pool.submit { })
            // The service must revoke publication synchronously before
            // trying a potentially saturated asynchronous cleanup pool.
            assertTrue(gate.cancel())
            assertFalse(pool.submit { error("cleanup queue is full") })
            assertFalse(gate.beginPublication())
        } finally {
            release.countDown()
            pool.shutdown()
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
