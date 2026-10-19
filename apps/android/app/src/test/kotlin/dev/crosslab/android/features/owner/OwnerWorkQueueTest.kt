package dev.crosslab.android.features.owner

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class OwnerWorkQueueTest {
    @Test
    fun boundedBackpressureRejectsExcessRatherThanRunningOnCallerThread() {
        val queue = OwnerWorkQueue(pendingCapacity = 1)
        val started = CountDownLatch(1)
        val release = CountDownLatch(1)
        val nextCompleted = CountDownLatch(1)
        val overflowExecuted = AtomicBoolean(false)
        try {
            assertTrue(queue.submit {
                started.countDown()
                release.await(5, TimeUnit.SECONDS)
            })
            assertTrue(started.await(5, TimeUnit.SECONDS))
            assertTrue(queue.submit { nextCompleted.countDown() })
            assertFalse(queue.submit { overflowExecuted.set(true) })
        } finally {
            release.countDown()
            queue.close()
        }
        assertTrue(nextCompleted.await(5, TimeUnit.SECONDS))
        assertFalse(overflowExecuted.get())
    }

    @Test
    fun rejectedWorkNeverRunsAfterOwnerScreenCloses() {
        val queue = OwnerWorkQueue(pendingCapacity = 1)
        queue.close()
        val executed = AtomicBoolean(false)
        assertFalse(queue.submit { executed.set(true) })
        assertFalse(executed.get())
    }

    @Test
    fun failedOwnerTaskDoesNotDiscardTheNextAcceptedCommand() {
        val queue = OwnerWorkQueue(pendingCapacity = 2)
        val completed = CountDownLatch(1)
        try {
            assertTrue(queue.submit { error("owner storage unavailable") })
            assertTrue(queue.submit { completed.countDown() })
            assertTrue(completed.await(5, TimeUnit.SECONDS))
        } finally {
            queue.close()
        }
    }
}
