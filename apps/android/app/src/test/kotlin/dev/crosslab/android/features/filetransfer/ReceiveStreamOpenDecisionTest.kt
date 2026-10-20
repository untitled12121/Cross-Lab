package dev.crosslab.android.features.filetransfer

import org.junit.Assert.assertEquals
import org.junit.Test

class ReceiveStreamOpenDecisionTest {
    @Test
    fun firstOpenMustMatchDurableCheckpoint() {
        assertEquals(
            ReceiveStreamOpenDecision.BIND,
            receiveStreamOpenDecision(false, 1_048_576uL, 1_048_576uL),
        )
        assertEquals(
            ReceiveStreamOpenDecision.REJECT,
            receiveStreamOpenDecision(false, 1uL, 1_048_576uL),
        )
    }

    @Test
    fun duplicateOpenCannotCancelAlreadyBoundReceiver() {
        // The current authenticated stream remains the only stream authority.
        assertEquals(
            ReceiveStreamOpenDecision.IGNORE,
            receiveStreamOpenDecision(true, 1_048_576uL, 1_048_576uL),
        )
        assertEquals(
            ReceiveStreamOpenDecision.IGNORE,
            receiveStreamOpenDecision(true, 0uL, 1_048_576uL),
        )
    }

    @Test
    fun freshZeroByteStreamStillBindsAtZero() {
        assertEquals(
            ReceiveStreamOpenDecision.BIND,
            receiveStreamOpenDecision(false, 0uL, 0uL),
        )
    }
}
