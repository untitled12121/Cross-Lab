package dev.crosslab.android.features.permissions

import java.nio.ByteBuffer
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class AndroidPolicyStoreBundleTest {
    @Test
    fun bundleRoundTripsWithoutExposingAdditionalState() {
        val anchor = ByteBuffer.allocate(40).putLong(7L).array()
        val expected =
            PolicyStoreBundle(
                envelope = byteArrayOf(1, 2, 3),
                anchor = anchor,
                anchorMac = byteArrayOf(4, 5, 6),
            )

        val decoded = PolicyStoreBundle.decode(expected.encode())

        assertArrayEquals(expected.envelope, decoded.envelope)
        assertArrayEquals(expected.anchor, decoded.anchor)
        assertArrayEquals(expected.anchorMac, decoded.anchorMac)
        assertEquals(7uL, decoded.anchorRevision())
    }

    @Test
    fun bundleRejectsTrailingData() {
        val encoded =
            PolicyStoreBundle(
                envelope = byteArrayOf(1),
                anchor = ByteBuffer.allocate(40).putLong(1L).array(),
                anchorMac = byteArrayOf(2),
            ).encode() + byteArrayOf(9)

        assertThrows(IllegalArgumentException::class.java) {
            PolicyStoreBundle.decode(encoded)
        }
    }
}
