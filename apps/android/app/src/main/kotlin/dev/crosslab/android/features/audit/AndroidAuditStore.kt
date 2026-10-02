package dev.crosslab.android.features.audit

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.nio.ByteBuffer
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.KeyGenerator
import javax.crypto.Mac
import javax.crypto.SecretKey
import uniffi.crosslab_mobile_ffi.MobileAuditAction
import uniffi.crosslab_mobile_ffi.MobileAuditHistory
import uniffi.crosslab_mobile_ffi.MobileAuditOutcome
import uniffi.crosslab_mobile_ffi.auditHistoryClear
import uniffi.crosslab_mobile_ffi.auditHistoryExport
import uniffi.crosslab_mobile_ffi.auditHistoryLoad
import uniffi.crosslab_mobile_ffi.auditHistoryRecord

/**
 * Separate owner-only audit state; no private payload, file path or peer ID is persisted.
 * Android Keystore anchors each version to reject old copies after commit.
 */
class AndroidAuditStore(context: Context) {
    private val stateFile =
        AtomicFile(
            context.noBackupFilesDir.resolve("crosslab/audit/history-v1.bin")
                .apply { parentFile?.mkdirs() },
        )
    private val keyStore = KeyStore.getInstance(KEY_STORE).apply { load(null) }

    @Synchronized
    fun read(): MobileAuditHistory {
        val previous = readBundle()
        val history = auditHistoryLoad(previous?.payload, currentHour())
        if (previous != null && !MessageDigest.isEqual(previous.payload, history.payload)) {
            // Expired entries must not remain on disk after the owner reads history.
            commit(previous, history.payload)
        }
        return history
    }

    @Synchronized
    fun record(
        action: MobileAuditAction,
        outcome: MobileAuditOutcome,
        revision: ULong = 0uL,
    ): MobileAuditHistory {
        val previous = readBundle()
        val now = currentHour()
        val next = auditHistoryRecord(previous?.payload, now, action, outcome, revision)
        commit(previous, next.payload)
        return next
    }

    @Synchronized
    fun clear(): MobileAuditHistory {
        val previous = readBundle()
        val next = auditHistoryClear(previous?.payload, currentHour())
        commit(previous, next.payload)
        return next
    }

    @Synchronized
    fun export(): String =
        auditHistoryExport(readBundle()?.payload, currentHour())

    private fun readBundle(): AuditBundle? {
        val aliases = aliases()
        if (!stateFile.baseFile.isFile) {
            if (aliases.isNotEmpty()) {
                throw AuditStoreUnavailable("committed audit history is missing")
            }
            return null
        }
        val bundle = AuditBundle.decode(stateFile.readFully())
        val alias = alias(bundle.revision)
        if (aliases.size != 1 || aliases.single() != alias) {
            throw AuditStoreUnavailable("audit currentness keys are inconsistent")
        }
        val key = keyStore.getKey(alias, null) as? SecretKey
            ?: throw AuditStoreUnavailable("audit currentness key unavailable")
        if (!MessageDigest.isEqual(sign(key, bundle.signedBytes()), bundle.mac)) {
            throw AuditStoreUnavailable("audit history integrity verification failed")
        }
        // A correctly signed envelope may still carry an unsupported or malformed payload.
        auditHistoryLoad(bundle.payload, currentHour())
        return bundle
    }

    private fun commit(previous: AuditBundle?, payload: ByteArray) {
        require(payload.size <= MAX_HISTORY_BYTES) { "audit payload too large" }
        val revision = (previous?.revision ?: 0L).let {
            if (it == Long.MAX_VALUE) throw AuditStoreUnavailable("audit revision exhausted")
            it + 1L
        }
        val keyAlias = alias(revision)
        if (keyStore.containsAlias(keyAlias)) {
            throw AuditStoreUnavailable("audit currentness revision already exists")
        }
        val key = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_HMAC_SHA256, KEY_STORE).run {
            init(
                KeyGenParameterSpec.Builder(
                    keyAlias,
                    KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY,
                )
                    .setDigests(KeyProperties.DIGEST_SHA256)
                    .setUserAuthenticationRequired(false)
                    .build(),
            )
            generateKey()
        }
        val unsigned = AuditBundle(revision, payload, ByteArray(MAC_BYTES))
        val next = unsigned.copy(mac = sign(key, unsigned.signedBytes()))
        val output = stateFile.startWrite()
        try {
            output.write(next.encode())
            output.fd.sync()
            stateFile.finishWrite(output)
        } catch (error: Throwable) {
            stateFile.failWrite(output)
            runCatching { keyStore.deleteEntry(keyAlias) }
            throw error
        }
        previous?.let { keyStore.deleteEntry(alias(it.revision)) }
    }

    private fun sign(key: SecretKey, message: ByteArray): ByteArray =
        Mac.getInstance("HmacSHA256").run {
            init(key)
            doFinal(message)
        }

    private fun aliases(): List<String> =
        keyStore.aliases().toList().filter { it.startsWith(KEY_PREFIX) }

    private fun alias(revision: Long): String = "$KEY_PREFIX$revision"

    companion object {
        private const val KEY_STORE = "AndroidKeyStore"
        private const val KEY_PREFIX = "crosslab.audit.anchor.v1."
        private const val MAX_HISTORY_BYTES = 256 * 1024
        private const val MAC_BYTES = 32
    }
}

class AuditStoreUnavailable(message: String) : IllegalStateException(message)

internal data class AuditBundle(
    val revision: Long,
    val payload: ByteArray,
    val mac: ByteArray,
) {
    fun signedBytes(): ByteArray =
        ByteBuffer.allocate(MAGIC.size + Long.SIZE_BYTES + Int.SIZE_BYTES + payload.size)
            .put(MAGIC)
            .putLong(revision)
            .putInt(payload.size)
            .put(payload)
            .array()

    fun encode(): ByteArray =
        signedBytes() + mac

    companion object {
        private val MAGIC = byteArrayOf(0x43, 0x4c, 0x41, 0x48, 0x01)
        private const val MAX_BYTES = 256 * 1024
        private const val MAC_BYTES = 32

        fun decode(bytes: ByteArray): AuditBundle {
            if (bytes.size < MAGIC.size + Long.SIZE_BYTES + Int.SIZE_BYTES + MAC_BYTES) {
                throw AuditStoreUnavailable("audit envelope truncated")
            }
            val input = ByteBuffer.wrap(bytes)
            val magic = ByteArray(MAGIC.size).also(input::get)
            if (!MessageDigest.isEqual(magic, MAGIC)) {
                throw AuditStoreUnavailable("audit envelope schema unsupported")
            }
            val revision = input.long
            val size = input.int
            if (revision <= 0 || size !in 0..MAX_BYTES || input.remaining() != size + MAC_BYTES) {
                throw AuditStoreUnavailable("audit envelope size/revision invalid")
            }
            val payload = ByteArray(size).also(input::get)
            val mac = ByteArray(MAC_BYTES).also(input::get)
            return AuditBundle(revision, payload, mac)
        }
    }
}

internal fun currentHour(): ULong =
    (System.currentTimeMillis() / 3_600_000L).coerceAtLeast(0).toULong()
