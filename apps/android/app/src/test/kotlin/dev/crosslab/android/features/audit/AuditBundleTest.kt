package dev.crosslab.android.features.audit

import java.io.FileNotFoundException
import java.io.IOException
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
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
    fun atomicBackupReadIsAttemptedWithoutPrecheckingBaseFile() {
        val bytes = byteArrayOf(3, 4, 5)
        var readAttempts = 0
        assertArrayEquals(
            bytes,
            readAtomicAuditBytes {
                readAttempts++
                bytes
            },
        )
        assertEquals(1, readAttempts)
    }

    @Test
    fun missingAuditAndUnavailableStorageAreNotConfused() {
        assertNull(readAtomicAuditBytes { throw FileNotFoundException("first run") })
        assertThrows(IOException::class.java) {
            readAtomicAuditBytes { throw IOException("storage failure") }
        }
    }

    @Test
    fun auditPayloadNeverIncludesPlatformIdentifiersImplicitly() {
        val bundle = AuditBundle(1, byteArrayOf(), ByteArray(32))
        assertEquals(5 + 8 + 4 + 32, bundle.encode().size)
    }
}
