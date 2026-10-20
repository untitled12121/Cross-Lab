package dev.crosslab.android.features.pairing

import dev.crosslab.android.features.workers.BoundedWorkerPool
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PairingWorkerIsolationTest {
    @Test
    fun cancellationLaneCanRunWhilePairingWorkerIsBlocked() {
        val pairing = BoundedWorkerPool("crosslab-test-pairing", 1, 1)
        val cancellations = BoundedWorkerPool("crosslab-test-cancel", 1, 1)
        val connecting = CountDownLatch(1)
        val release = CountDownLatch(1)
        val cancelled = CountDownLatch(1)
        try {
            assertTrue(pairing.submit {
                connecting.countDown()
                release.await(5, TimeUnit.SECONDS)
            })
            assertTrue(connecting.await(5, TimeUnit.SECONDS))
            assertTrue(cancellations.submit { cancelled.countDown() })
            assertTrue(cancelled.await(5, TimeUnit.SECONDS))
        } finally {
            release.countDown()
            pairing.shutdownNow()
            cancellations.shutdown()
        }
    }

    @Test
    fun pairingStormHasFiniteBacklogAndReportsAdmissionFailure() {
        val pairing = BoundedWorkerPool("crosslab-test-pairing", 1, 1)
        val connecting = CountDownLatch(1)
        val release = CountDownLatch(1)
        try {
            assertTrue(pairing.submit {
                connecting.countDown()
                release.await(5, TimeUnit.SECONDS)
            })
            assertTrue(connecting.await(5, TimeUnit.SECONDS))
            assertTrue(pairing.submit { })
            assertFalse(pairing.submit { error("must never launch unadmitted pairing") })
        } finally {
            release.countDown()
            pairing.shutdownNow()
        }
    }
}
