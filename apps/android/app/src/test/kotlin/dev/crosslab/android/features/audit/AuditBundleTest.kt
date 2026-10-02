package dev.crosslab.android.features.audit

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class AuditBundleTest {
    @Test
    fun bundleRoundTripsAndRejectsCorruptEnvelopeBounds() {
        val bundle = AuditBundle(7, byteArrayOf(1, 2, 3), ByteArray(32) { 0x45 })
        val restored = AuditBundle.decode(bundle.encode())
        assertEquals(7, restored.revision)
        assertArrayEquals(bundle.payload, restored.payload)
        assertArrayEquals(bundle.mac, restored.mac)
        assertThrows(AuditStoreUnavailable::class.java) {
            AuditBundle.decode(bundle.encode() + byteArrayOf(1))
        }
        assertThrows(AuditStoreUnavailable::class.java) {
            AuditBundle.decode(ByteArray(1))
        }
        val tampered = bundle.encode().also { it[0] = 99 }
        assertThrows(AuditStoreUnavailable::class.java) {
            AuditBundle.decode(tampered)
        }
    }

    @Test
    fun auditPayloadNeverIncludesPlatformIdentifiersImplicitly() {
        val bundle = AuditBundle(1, byteArrayOf(), ByteArray(32))
        assertEquals(5 + 8 + 4 + 32, bundle.encode().size)
    }
}
