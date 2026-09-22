package dev.crosslab.android.features.devices

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TrustedSessionNsdDiscoveryTest {
    @Test
    fun txtProfileAcceptsOnlyExactVersionAttribute() {
        assertTrue(
            hasExactTrustedSessionTxtProfile(
                attributes = mapOf("v" to byteArrayOf('1'.code.toByte())),
                key = "v",
                version = "1",
            ),
        )
        assertFalse(
            hasExactTrustedSessionTxtProfile(
                attributes =
                    mapOf(
                        "v" to byteArrayOf('1'.code.toByte()),
                        "device" to byteArrayOf(1),
                    ),
                key = "v",
                version = "1",
            ),
        )
        assertFalse(
            hasExactTrustedSessionTxtProfile(
                attributes = mapOf("v" to byteArrayOf('2'.code.toByte())),
                key = "v",
                version = "1",
            ),
        )
    }
}
