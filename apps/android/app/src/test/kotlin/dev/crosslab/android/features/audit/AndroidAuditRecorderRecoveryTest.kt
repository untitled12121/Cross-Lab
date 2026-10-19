package dev.crosslab.android.features.audit

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.crosslab_mobile_ffi.MobileAuditAction
import uniffi.crosslab_mobile_ffi.MobileAuditHistory
import uniffi.crosslab_mobile_ffi.MobileAuditOutcome

class AndroidAuditRecorderRecoveryTest {
    private class FailingAuditStore : OwnerAuditStore {
        val reads = AtomicInteger()

        override fun read(): MobileAuditHistory {
            reads.incrementAndGet()
            throw IllegalStateException("protected audit store unavailable")
        }

        override fun record(
            action: MobileAuditAction,
            outcome: MobileAuditOutcome,
            revision: ULong,
        ): MobileAuditHistory = throw IllegalStateException("unavailable")

        override fun clear(): MobileAuditHistory = throw IllegalStateException("unavailable")
        override fun export(): String = throw IllegalStateException("unavailable")
    }

    @Test
    fun unavailable_store_can_be_retried_without_claiming_history_recovered() {
        val store = FailingAuditStore()
        val recorder = AndroidAuditRecorder(store)
        val initialUnavailable = CountDownLatch(1)
        val subscription =
            recorder.observe { status ->
                if (status.unavailable && status.history == null) initialUnavailable.countDown()
            }
        try {
            assertTrue(initialUnavailable.await(5, TimeUnit.SECONDS))
            val retryDone = CountDownLatch(1)
            val retryFailed = java.util.concurrent.atomic.AtomicBoolean(false)
            recorder.refresh { result ->
                retryFailed.set(result.isFailure)
                retryDone.countDown()
            }
            assertTrue(retryDone.await(5, TimeUnit.SECONDS))
            assertTrue(retryFailed.get())
            assertEquals(2, store.reads.get())
        } finally {
            subscription.close()
            recorder.close()
        }
    }
}
