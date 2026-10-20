package dev.crosslab.android.features.storage

import android.util.AtomicFile
import java.io.ByteArrayOutputStream
import java.io.InputStream

/** Read private state with a hard allocation bound, including if the file grows mid-read. */
internal fun AtomicFile.readBounded(maxBytes: Int): ByteArray =
    openRead().use { readBoundedState(it, maxBytes) }

internal fun readBoundedState(input: InputStream, maxBytes: Int): ByteArray {
    require(maxBytes >= 0)
    val result = ByteArrayOutputStream()
    val buffer = ByteArray(8 * 1024)
    while (true) {
        val count = input.read(buffer)
        if (count < 0) return result.toByteArray()
        if (count > maxBytes - result.size()) {
            throw IllegalStateException("protected state file exceeds its allowed size")
        }
        result.write(buffer, 0, count)
    }
}
