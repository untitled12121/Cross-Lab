package dev.crosslab.android.features.filetransfer

import java.io.ByteArrayInputStream
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class AndroidFileTransferSourceReaderTest {
    @Test
    fun reader_returns_bounded_chunks_and_exact_eof() {
        val bytes = ByteArray(70_000) { index -> (index % 251).toByte() }
        val reader =
            AndroidFileTransferSourceReader(
                input = ByteArrayInputStream(bytes),
                remaining = bytes.size.toLong(),
            )

        val first = checkNotNull(reader.readChunk())
        val second = checkNotNull(reader.readChunk())

        assertEquals(64 * 1024, first.size)
        assertEquals(bytes.size - first.size, second.size)
        assertArrayEquals(bytes.copyOfRange(0, first.size), first)
        assertArrayEquals(bytes.copyOfRange(first.size, bytes.size), second)
        assertNull(reader.readChunk())
        reader.close()
    }

    @Test
    fun document_provider_zero_reads_are_bounded_and_retryable() {
        val provider =
            object : ByteArrayInputStream(byteArrayOf(1, 2, 3)) {
                var zeroReads = 0

                override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
                    if (zeroReads++ < 2) return 0
                    return super.read(buffer, offset, length)
                }
            }
        val reader = AndroidFileTransferSourceReader(provider, 3)
        assertArrayEquals(byteArrayOf(1, 2, 3), checkNotNull(reader.readChunk()))
        assertNull(reader.readChunk())
        reader.close()
    }

    @Test
    fun stalled_document_provider_fails_without_unbounded_spin() {
        var attempts = 0
        val provider =
            object : ByteArrayInputStream(byteArrayOf(1)) {
                override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
                    attempts++
                    return 0
                }
            }
        val reader = AndroidFileTransferSourceReader(provider, 1)
        assertThrows(FileTransferStorageUnavailable::class.java) { reader.readChunk() }
        assertEquals(3, attempts)
        reader.close()
    }

    @Test
    fun reader_fails_closed_when_source_shortens_after_prepare() {
        val reader =
            AndroidFileTransferSourceReader(
                input = ByteArrayInputStream(byteArrayOf(1, 2, 3)),
                remaining = 4,
            )

        assertThrows(FileTransferStorageUnavailable::class.java) {
            reader.readChunk()
        }
        reader.close()
    }
}
