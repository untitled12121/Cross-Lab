package dev.crosslab.android.features.pairing

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PairingScanGateTest {
    @Test
    fun oneInvitationIsAcceptedPerScannerLifetime() {
        val gate = PairingScanGate()

        assertTrue(gate.canAnalyze())
        assertTrue(gate.tryAccept())
        assertFalse(gate.canAnalyze())
        assertFalse(gate.tryAccept())
    }

    @Test
    fun callbacksCannotAcceptAnInvitationAfterDismissal() {
        val gate = PairingScanGate()

        gate.close()
        gate.close()
        assertFalse(gate.isOpen())
        assertFalse(gate.canAnalyze())
        assertFalse(gate.tryAccept())
    }

    @Test
    fun anAcceptedScanCannotBeDeliveredAfterDismissal() {
        val gate = PairingScanGate()

        assertTrue(gate.tryAccept())
        gate.close()
        assertFalse(gate.isOpen())
    }
}
