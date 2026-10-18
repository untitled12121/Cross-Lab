package dev.crosslab.android.features.identity

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertNotSame
import org.junit.Test

class RevocationSelectionTest {
    @Test
    fun refreshedInventoryCanNeverRevokeAReorderedDevice() {
        val peerA = ByteArray(32) { 1 }
        val peerB = ByteArray(32) { 2 }
        val reordered = listOf(peerB, peerA)
        assertNull(revocationTarget(reordered, 0, 7, 8))
        assertNull(revocationTarget(reordered, -1, 8, 8))
        assertNull(revocationTarget(reordered, 2, 8, 8))
    }

    @Test
    fun selectedTargetIsTheFullCopiedIdentifier() {
        val original = ByteArray(32) { 3 }
        val selected = revocationTarget(listOf(original), 0, 5, 5)!!
        assertArrayEquals(original, selected)
        assertNotSame(original, selected)
        original.fill(0)
        assertArrayEquals(ByteArray(32) { 3 }, selected)
    }
}
