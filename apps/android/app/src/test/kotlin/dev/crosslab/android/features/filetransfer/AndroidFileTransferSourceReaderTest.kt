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
