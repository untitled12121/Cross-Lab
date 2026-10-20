package dev.crosslab.android.features.filetransfer

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DeferredTransferCleanupTest {
    @Test
    fun rejectedCleanupRetainsTheSameReceiverUntilLaterAdmission() {
        val pending = DeferredTransferCleanup<Any>()
        val receiver = Any()
        assertTrue(pending.retain(receiver))
        assertTrue(pending.hasPending())
        assertFalse(pending.trySubmit { false })
        assertTrue(pending.hasPending())

        var admitted: Any? = null
        assertTrue(pending.trySubmit { item -> admitted = item; true })
        assertTrue(admitted === receiver)
        assertFalse(pending.hasPending())
        assertTrue(pending.trySubmit { error("no cleanup was retained") })
    }

    @Test
    fun aDifferentReceiverCannotReplaceUndrainedCleanup() {
        val pending = DeferredTransferCleanup<Any>()
        val first = Any()
        assertTrue(pending.retain(first))
        assertTrue(pending.retain(first))
        assertFalse(pending.retain(Any()))
        assertTrue(pending.trySubmit { it === first })
        assertFalse(pending.hasPending())
    }
}
