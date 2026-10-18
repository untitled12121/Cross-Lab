package dev.crosslab.android.features.pairing

import dev.crosslab.android.features.networking.androidNsdServiceType
import java.net.InetAddress
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PairingNsdDiscoveryProfileTest {
    @Test
    fun ipv4RouteIsPreferredOverUnreachableIpv6ForIpv4Listener() {
        val ipv6 = InetAddress.getByAddress(ByteArray(16))
        val ipv4 = InetAddress.getByAddress(byteArrayOf(192.toByte(), 0, 2, 1))
        assertEquals(ipv4, firstIpv4(listOf(ipv6, ipv4)))
        assertNull(firstIpv4(listOf(ipv6)))
    }

    @Test
    fun androidServiceTypeKeepsDnsSdTypeAndDropsLocalDomain() {
        assertEquals(
            "_crosslab-pair._udp.",
            androidNsdServiceType("_crosslab-pair._udp.local."),
        )
        assertNull(androidNsdServiceType("_crosslab-pair._udp."))
    }

    @Test
    fun txtProfileAcceptsOnlyVersionOne() {
        assertTrue(hasExactPairingTxtProfile(mapOf("v" to byteArrayOf('1'.code.toByte()))))
        assertFalse(hasExactPairingTxtProfile(emptyMap()))
        assertFalse(hasExactPairingTxtProfile(mapOf("v" to byteArrayOf('2'.code.toByte()))))
        assertFalse(
            hasExactPairingTxtProfile(
                mapOf(
                    "v" to byteArrayOf('1'.code.toByte()),
                    "device" to byteArrayOf(1),
                ),
            ),
        )
    }
}
