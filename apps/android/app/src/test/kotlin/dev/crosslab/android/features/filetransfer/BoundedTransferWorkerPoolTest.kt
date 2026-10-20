package dev.crosslab.android.features.filetransfer

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import kotlin.concurrent.thread
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BoundedTransferWorkerPoolTest {
    @Test
    fun saturatedQueueRejectsWithoutEverRunningWorkOnUiCaller() {
        val pool = BoundedTransferWorkerPool("crosslab-test-transfer", 1, 1)
        val started = CountDownLatch(1)
        val release = CountDownLatch(1)
        val queued = CountDownLatch(1)
        val rejectedExecuted = AtomicBoolean(false)
        try {
            assertTrue(pool.submit {
                started.countDown()
                release.await(5, TimeUnit.SECONDS)
            })
            assertTrue(started.await(5, TimeUnit.SECONDS))
            assertTrue(pool.submit { queued.countDown() })
            assertFalse(pool.submit { rejectedExecuted.set(true) })
            assertFalse(rejectedExecuted.get())
        } finally {
            release.countDown()
            pool.shutdown()
        }
        assertTrue(queued.await(5, TimeUnit.SECONDS))
        assertFalse(rejectedExecuted.get())
    }

    @Test
    fun shutdownRejectsNewSubmissionsAndDrainsAcceptedWork() {
        val pool = BoundedTransferWorkerPool("crosslab-test-transfer", 1, 2)
        val complete = CountDownLatch(2)
        assertTrue(pool.submit { complete.countDown() })
        assertTrue(pool.submit { complete.countDown() })
        pool.shutdown()
        assertFalse(pool.submit { error("unexpected post-close task") })
        assertTrue(complete.await(5, TimeUnit.SECONDS))
    }

    @Test
    fun brokenNativeTaskCannotBlockLaterAcceptedWorker() {
        val pool = BoundedTransferWorkerPool("crosslab-test-transfer", 1, 2)
        val done = CountDownLatch(1)
        try {
            assertTrue(pool.submit { error("simulated broken native callback") })
            assertTrue(pool.submit { done.countDown() })
            assertTrue(done.await(5, TimeUnit.SECONDS))
        } finally {
            pool.shutdown()
        }
    }

    @Test
    fun concurrentOverloadStillCapsWorkerCountAndAcceptedBacklog() {
        val pool = BoundedTransferWorkerPool("crosslab-test-transfer", 1, 2)
        val started = CountDownLatch(1)
        val release = CountDownLatch(1)
        val attempted = CountDownLatch(10)
        val accepted = AtomicInteger(0)
        try {
            assertTrue(pool.submit {
                started.countDown()
                release.await(5, TimeUnit.SECONDS)
            })
            assertTrue(started.await(5, TimeUnit.SECONDS))
            val callers = (0 until 10).map {
                thread {
                    if (pool.submit { }) accepted.incrementAndGet()
                    attempted.countDown()
                }
            }
            assertTrue(attempted.await(5, TimeUnit.SECONDS))
            callers.forEach { it.join(5_000) }
            assertEquals(2, accepted.get())
        } finally {
            release.countDown()
            pool.shutdown()
        }
    }
}
