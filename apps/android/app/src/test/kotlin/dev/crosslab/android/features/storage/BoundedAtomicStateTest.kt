package dev.crosslab.android.features.storage

import java.io.ByteArrayInputStream
import java.io.InputStream
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class BoundedAtomicStateTest {
    @Test
    fun acceptsExactLimitButRejectsAnyExtraByte() {
        val payload = ByteArray(8_193) { (it % 251).toByte() }
        assertArrayEquals(payload, readBoundedState(ByteArrayInputStream(payload), payload.size))
        assertThrows(IllegalStateException::class.java) {
            readBoundedState(ByteArrayInputStream(payload), payload.size - 1)
        }
        assertThrows(IllegalStateException::class.java) {
            readBoundedState(ByteArrayInputStream(byteArrayOf(1)), 0)
        }
    }

    @Test
    fun doesNotReadUnboundedBytesFromHostileState() {
        var delivered = 0
        val input = object : InputStream() {
            override fun read(): Int {
                delivered++
                return 42
            }

            override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
                val count = minOf(length, 512)
                buffer.fill(42, offset, offset + count)
                delivered += count
                return count
            }
        }
        assertThrows(IllegalStateException::class.java) {
            readBoundedState(input, 1_024)
        }
        // A growing file can never force an arbitrarily large allocation/read.
        org.junit.Assert.assertTrue(delivered <= 1_024 + 8 * 1_024)
    }
}
