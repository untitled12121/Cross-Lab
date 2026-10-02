package dev.crosslab.android.features.filetransfer

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
    fun publication_commit_rejects_late_cancellation() {
        val gate = TransferPublicationGate()
        assertTrue(gate.beginPublication())
        assertFalse(gate.cancel())
        assertFalse(gate.cancelled())
        assertFalse(gate.beginPublication())
    }
}
