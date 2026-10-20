package dev.crosslab.android.features.filetransfer

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeferredTransferCleanupTest {
    @Test
    fun rejectedCleanupRetainsReceiverThroughAdmissionAndCompletion() {
        val pending = DeferredTransferCleanup<Any>()
        val receiver = Any()
        assertTrue(pending.retain(receiver))
        assertFalse(pending.trySubmit { false })
        assertTrue(pending.hasPending())

        var accepted: Any? = null
        assertTrue(pending.trySubmit { item -> accepted = item; true })
        assertTrue(accepted === receiver)
        assertTrue(pending.hasPending())
        // Repeat submission must never enqueue the same file close twice.
        assertTrue(pending.trySubmit { error("duplicate receiver cleanup") })
        assertFalse(pending.retain(Any()))
        assertTrue(pending.complete(receiver))
        assertFalse(pending.hasPending())
        assertTrue(pending.trySubmit { error("no cleanup pending") })
    }

    @Test
    fun staleCompletionCannotReleaseAnotherReceiversCloseGate() {
        val pending = DeferredTransferCleanup<Any>()
        val first = Any()
        val second = Any()
        assertTrue(pending.retain(first))
        assertTrue(pending.trySubmit { true })
        assertFalse(pending.complete(second))
        assertTrue(pending.hasPending())
        assertTrue(pending.complete(first))
        assertTrue(pending.retain(second))
        assertTrue(pending.trySubmit { true })
        assertFalse(pending.complete(first))
        assertTrue(pending.hasPending())
        assertTrue(pending.complete(second))
    }

    @Test
    fun shutdownTransfersOnlyRejectedWorkToTerminalClose() {
        val pending = DeferredTransferCleanup<Any>()
        val first = Any()
        assertTrue(pending.retain(first))
        assertFalse(pending.trySubmit { false })
        assertTrue(pending.takeUnsubmitted() === first)
        assertFalse(pending.hasPending())

        val accepted = Any()
        assertTrue(pending.retain(accepted))
        assertTrue(pending.trySubmit { true })
        assertNull(pending.takeUnsubmitted())
        assertTrue(pending.hasPending())
        assertTrue(pending.complete(accepted))
    }

    @Test
    fun actualWorkerCompletionNotJustSubmissionReleasesCloseGate() {
        val lock = Any()
        val pending = DeferredTransferCleanup<Any>()
        val worker = BoundedTransferWorkerPool("crosslab-cleanup-test", 1, 1)
        val started = CountDownLatch(1)
        val finish = CountDownLatch(1)
        val done = CountDownLatch(1)
        val receiver = Any()
        try {
            synchronized(lock) {
                assertTrue(pending.retain(receiver))
                assertTrue(pending.trySubmit { item ->
                    worker.submit {
                        started.countDown()
                        finish.await(5, TimeUnit.SECONDS)
                        synchronized(lock) { pending.complete(item) }
                        done.countDown()
                    }
                })
                assertTrue(pending.hasPending())
            }
            assertTrue(started.await(5, TimeUnit.SECONDS))
            assertFalse(synchronized(lock) { pending.retain(Any()) })
            finish.countDown()
            assertTrue(done.await(5, TimeUnit.SECONDS))
            assertFalse(synchronized(lock) { pending.hasPending() })
        } finally {
            finish.countDown()
            worker.shutdown()
        }
    }
}
