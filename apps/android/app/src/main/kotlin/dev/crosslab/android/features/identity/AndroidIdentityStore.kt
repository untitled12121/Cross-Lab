package dev.crosslab.android.features.identity

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.KeyGenerator
import javax.crypto.Mac
import javax.crypto.SecretKey
import uniffi.crosslab_mobile_ffi.identityStorePayload
import uniffi.crosslab_mobile_ffi.identityStorePrepareCommit
import uniffi.crosslab_mobile_ffi.identityStoreValidate

class AndroidIdentityStore(
    context: Context,
) {
    private val stateFile =
        AtomicFile(
            context.noBackupFilesDir
                .resolve("crosslab/identity/store-v1.bin")
                .apply { parentFile?.mkdirs() },
        )
    private val keyStore =
        KeyStore.getInstance(ANDROID_KEY_STORE).apply {
            load(null)
        }

    @Synchronized
    fun loadPayload(): ByteArray? {
        if (!stateFile.baseFile.isFile) return null

        val bundle = IdentityStoreBundle.decode(stateFile.readFully())
        val revision = bundle.anchorRevision()
        val alias = anchorAlias(revision)
        val key =
            keyStore.getKey(alias, null) as? SecretKey
                ?: throw IdentityStoreUnavailable("identity currentness key is missing")

        val expectedMac = mac(key, bundle.anchor)
        if (!MessageDigest.isEqual(expectedMac, bundle.anchorMac)) {
            throw IdentityStoreUnavailable("identity currentness authentication failed")
        }

        val validatedRevision = identityStoreValidate(bundle.envelope, bundle.anchor)
        if (validatedRevision.toLong() != revision) {
            throw IdentityStoreUnavailable("identity revision is inconsistent")
        }

        return identityStorePayload(bundle.envelope, bundle.anchor)
    }

    @Synchronized
    fun commitPayload(payload: ByteArray) {
        val current = loadBundleOrNull()
        val commit = identityStorePrepareCommit(current?.envelope, payload)
        val revision = commit.revision.toLong()
        val alias = anchorAlias(revision)

        if (keyStore.containsAlias(alias)) {
            keyStore.deleteEntry(alias)
        }

        val key = createAnchorKey(alias)
        val bundle =
            IdentityStoreBundle(
                envelope = commit.envelope,
                anchor = commit.anchor,
                anchorMac = mac(key, commit.anchor),
            )

        val output = stateFile.startWrite()
        try {
            output.write(bundle.encode())
            output.fd.sync()
            stateFile.finishWrite(output)
        } catch (error: Throwable) {
            stateFile.failWrite(output)
            runCatching { keyStore.deleteEntry(alias) }
            throw error
        }

        current?.anchorRevision()?.let { previous ->
            if (previous != revision) {
                runCatching { keyStore.deleteEntry(anchorAlias(previous)) }
            }
        }
        deleteOrphanAnchorKeys(revision)
    }

    @Synchronized
    fun wipe() {
        stateFile.delete()
        keyStore.aliases().toList()
            .filter { it.startsWith(ANCHOR_ALIAS_PREFIX) }
            .forEach(keyStore::deleteEntry)
    }

    private fun loadBundleOrNull(): IdentityStoreBundle? {
        if (!stateFile.baseFile.isFile) return null
        val bundle = IdentityStoreBundle.decode(stateFile.readFully())
        val revision = bundle.anchorRevision()
        val alias = anchorAlias(revision)
        val key =
            keyStore.getKey(alias, null) as? SecretKey
                ?: throw IdentityStoreUnavailable("identity currentness key is missing")
        if (!MessageDigest.isEqual(mac(key, bundle.anchor), bundle.anchorMac)) {
            throw IdentityStoreUnavailable("identity currentness authentication failed")
        }
        identityStoreValidate(bundle.envelope, bundle.anchor)
        return bundle
    }

    private fun createAnchorKey(alias: String): SecretKey {
        val generator =
            KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_HMAC_SHA256,
                ANDROID_KEY_STORE,
            )
        generator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY,
            )
                .setDigests(KeyProperties.DIGEST_SHA256)
                .setUserAuthenticationRequired(false)
                .build(),
        )
        return generator.generateKey()
    }

    private fun mac(
        key: SecretKey,
        anchor: ByteArray,
    ): ByteArray =
        Mac.getInstance(HMAC_SHA256).run {
            init(key)
            doFinal(anchor)
        }

    private fun deleteOrphanAnchorKeys(currentRevision: Long) {
        keyStore.aliases().toList()
            .filter { alias ->
                alias.startsWith(ANCHOR_ALIAS_PREFIX) &&
                    alias != anchorAlias(currentRevision)
            }
            .forEach { alias ->
                runCatching { keyStore.deleteEntry(alias) }
            }
    }

    private fun anchorAlias(revision: Long): String = "$ANCHOR_ALIAS_PREFIX$revision"

    companion object {
        private const val ANDROID_KEY_STORE = "AndroidKeyStore"
        private const val HMAC_SHA256 = "HmacSHA256"
        private const val ANCHOR_ALIAS_PREFIX = "crosslab.identity.anchor.v1."
    }
}

class IdentityStoreUnavailable(
    message: String,
) : IllegalStateException(message)

internal data class IdentityStoreBundle(
    val envelope: ByteArray,
    val anchor: ByteArray,
    val anchorMac: ByteArray,
) {
    fun anchorRevision(): Long {
        require(anchor.size >= Long.SIZE_BYTES) { "identity anchor is malformed" }
        return ByteBuffer.wrap(anchor, 0, Long.SIZE_BYTES).long
    }

    fun encode(): ByteArray {
        require(envelope.size <= MAX_FIELD_SIZE)
        require(anchor.size <= MAX_FIELD_SIZE)
        require(anchorMac.size <= MAX_FIELD_SIZE)

        return ByteArrayOutputStream().use { output ->
            output.write(MAGIC)
            writeField(output, envelope)
            writeField(output, anchor)
            writeField(output, anchorMac)
            output.toByteArray()
        }
    }

    companion object {
        private val MAGIC = byteArrayOf(0x43, 0x4c, 0x49, 0x53, 0x01)
        private const val MAX_FIELD_SIZE = 16 * 1024 * 1024

        fun decode(bytes: ByteArray): IdentityStoreBundle {
            val buffer = ByteBuffer.wrap(bytes)
            val magic = ByteArray(MAGIC.size)
            require(buffer.remaining() >= magic.size) { "identity store bundle is truncated" }
            buffer.get(magic)
            require(magic.contentEquals(MAGIC)) { "identity store bundle version is unsupported" }

            val envelope = readField(buffer)
            val anchor = readField(buffer)
            val mac = readField(buffer)
            require(!buffer.hasRemaining()) { "identity store bundle has trailing data" }

            return IdentityStoreBundle(envelope, anchor, mac)
        }

        private fun writeField(
            output: ByteArrayOutputStream,
            value: ByteArray,
        ) {
            output.write(ByteBuffer.allocate(Int.SIZE_BYTES).putInt(value.size).array())
            output.write(value)
        }

        private fun readField(buffer: ByteBuffer): ByteArray {
            require(buffer.remaining() >= Int.SIZE_BYTES) { "identity store field is truncated" }
            val size = buffer.int
            require(size in 0..MAX_FIELD_SIZE) { "identity store field is oversized" }
            require(buffer.remaining() >= size) { "identity store field is truncated" }
            return ByteArray(size).also(buffer::get)
        }
    }
}
