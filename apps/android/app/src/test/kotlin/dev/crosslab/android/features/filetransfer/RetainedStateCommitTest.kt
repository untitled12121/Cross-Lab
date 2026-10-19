package dev.crosslab.android.features.filetransfer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class RetainedStateCommitTest {
    private data class Counters(var offset: Int, var complete: Boolean = false)

    @Test
    fun successfulMutationBecomesVisibleOnlyAfterPersistence() {
        val durable = Counters(1024)
        var persisted = false
        val (next, result) =
            persistThenApply(
                current = durable,
                snapshot = { it.copy() },
                edit = { state ->
                    state.offset = 2048
                    state.offset
                },
                persist = { candidate ->
                    assertEquals(1024, durable.offset)
                    assertEquals(2048, candidate.offset)
                    persisted = true
                },
            )
        assertTrue(persisted)
        assertEquals(2048, result)
        assertEquals(2048, next.offset)
        assertEquals(1024, durable.offset)
    }

    @Test
    fun rejectedStorageWriteLeavesCurrentStateUntouched() {
        val current = Counters(256)
        assertThrows(IllegalStateException::class.java) {
            persistThenApply(
                current = current,
                snapshot = { it.copy() },
                edit = { it.offset = 512 },
                persist = { throw IllegalStateException("disk write failed") },
            )
        }
        assertEquals(256, current.offset)
        assertEquals(false, current.complete)
    }

    @Test
    fun mutationFailureDoesNotAttemptPersistence() {
        val current = Counters(100)
        var persisted = false
        assertThrows(IllegalArgumentException::class.java) {
            persistThenApply(
                current = current,
                snapshot = { it.copy() },
                edit = {
                    it.complete = true
                    throw IllegalArgumentException("invalid checkpoint")
                },
                persist = { persisted = true },
            )
        }
        assertEquals(false, persisted)
        assertEquals(false, current.complete)
    }
}
