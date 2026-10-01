package dev.crosslab.android.features.filetransfer

import java.io.ByteArrayInputStream
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FileTransferResumeTest {
    @Test
    fun durableOffsetAdvancesOnlyAtCheckpointOrFinalByte() {
        assertEquals(0L, durableOffset(64 * 1024L, 2_000_000L))
        assertEquals(1_048_576L, durableOffset(1_048_576L + 17L, 2_000_000L))
        assertEquals(2_000_000L, durableOffset(2_000_000L, 2_000_000L))
    }

    @Test
    fun skipExactlyConsumesResumePrefixAndFailsClosedWhenSourceIsShort() {
        val source = ByteArrayInputStream(byteArrayOf(1, 2, 3, 4, 5))
        assertTrue(skipExactly(source, 3uL))
        assertEquals(4, source.read())

        assertFalse(skipExactly(ByteArrayInputStream(byteArrayOf(1, 2)), 3uL))
    }
}
