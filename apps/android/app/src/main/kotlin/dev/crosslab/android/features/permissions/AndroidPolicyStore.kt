package dev.crosslab.android.features.permissions

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
import uniffi.crosslab_mobile_ffi.policyStoreValidate

class AndroidPolicyStore(
    context: Context,
) {
    private val stateFile =
        AtomicFile(
            context.noBackupFilesDir
                .resolve("crosslab/policy/store-v1.bin")
                .apply { parentFile?.mkdirs() },
        )
    private val keyStore =
        KeyStore.getInstance(ANDROID_KEY_STORE).apply {
            load(null)
        }

    @Synchronized
    fun load(): PolicyStoreBundle? {
        if (!stateFile.baseFile.isFile) {
            if (hasAnchorKeys()) {
                throw PolicyStoreUnavailable("committed policy state is missing")
            }
            return null
        }
        return loadRequired()
    }

    @Synchronized
    fun commit(
        expectedRevision: ULong,
        envelope: ByteArray,
        anchor: ByteArray,
    ) {
        val current = load()
        val currentRevision = current?.anchorRevision() ?: 0uL
        if (currentRevision != expectedRevision) {
            throw PolicyStoreUnavailable("policy revision changed concurrently")
        }

        val revision = policyStoreValidate(envelope, anchor)
        if (revision <= expectedRevision) {
            throw PolicyStoreUnavailable("policy revision did not advance")
        }

        val alias = anchorAlias(revision)
        if (keyStore.containsAlias(alias)) {
            keyStore.deleteEntry(alias)
        }

        val key = createAnchorKey(alias)
        val bundle =
            PolicyStoreBundle(
                envelope = envelope,
                anchor = anchor,
                anchorMac = mac(key, anchor),
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

    private fun loadRequired(): PolicyStoreBundle {
        val bundle = PolicyStoreBundle.decode(stateFile.readFully())
        val revision = bundle.anchorRevision()
        val alias = anchorAlias(revision)
        val aliases = anchorAliases()
        if (aliases.size != 1 || aliases.single() != alias) {
            throw PolicyStoreUnavailable("policy currentness key set is inconsistent")
        }
        val key =
            keyStore.getKey(alias, null) as? SecretKey
                ?: throw PolicyStoreUnavailable("policy currentness key is missing")

        if (!MessageDigest.isEqual(mac(key, bundle.anchor), bundle.anchorMac)) {
            throw PolicyStoreUnavailable("policy currentness authentication failed")
        }
        if (policyStoreValidate(bundle.envelope, bundle.anchor) != revision) {
            throw PolicyStoreUnavailable("policy revision is inconsistent")
        }
        return bundle
    }

    private fun hasAnchorKeys(): Boolean = anchorAliases().isNotEmpty()

    private fun anchorAliases(): List<String> =
        keyStore.aliases().toList().filter { it.startsWith(ANCHOR_ALIAS_PREFIX) }

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

    private fun deleteOrphanAnchorKeys(currentRevision: ULong) {
        keyStore.aliases().toList()
            .filter { alias ->
                alias.startsWith(ANCHOR_ALIAS_PREFIX) &&
                    alias != anchorAlias(currentRevision)
            }
            .forEach(keyStore::deleteEntry)
    }

    private fun anchorAlias(revision: ULong): String = "$ANCHOR_ALIAS_PREFIX$revision"

    companion object {
        private const val ANDROID_KEY_STORE = "AndroidKeyStore"
        private const val HMAC_SHA256 = "HmacSHA256"
        private const val ANCHOR_ALIAS_PREFIX = "crosslab.policy.anchor.v1."
    }
}

class PolicyStoreUnavailable(
    message: String,
) : IllegalStateException(message)

data class PolicyStoreBundle(
    val envelope: ByteArray,
    val anchor: ByteArray,
    val anchorMac: ByteArray,
) {
    fun anchorRevision(): ULong {
        require(anchor.size >= Long.SIZE_BYTES) { "policy anchor is malformed" }
        return ByteBuffer.wrap(anchor, 0, Long.SIZE_BYTES).long.toULong()
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
        private val MAGIC = byteArrayOf(0x43, 0x4c, 0x50, 0x53, 0x01)
        private const val MAX_FIELD_SIZE = 2 * 1024 * 1024

        fun decode(bytes: ByteArray): PolicyStoreBundle {
            val buffer = ByteBuffer.wrap(bytes)
            val magic = ByteArray(MAGIC.size)
            require(buffer.remaining() >= magic.size) { "policy store bundle is truncated" }
            buffer.get(magic)
            require(magic.contentEquals(MAGIC)) { "policy store bundle version is unsupported" }

            val envelope = readField(buffer)
            val anchor = readField(buffer)
            val mac = readField(buffer)
            require(!buffer.hasRemaining()) { "policy store bundle has trailing data" }

            return PolicyStoreBundle(envelope, anchor, mac)
        }

        private fun writeField(
            output: ByteArrayOutputStream,
            value: ByteArray,
        ) {
            output.write(ByteBuffer.allocate(Int.SIZE_BYTES).putInt(value.size).array())
            output.write(value)
        }

        private fun readField(buffer: ByteBuffer): ByteArray {
            require(buffer.remaining() >= Int.SIZE_BYTES) { "policy store field is truncated" }
            val size = buffer.int
            require(size in 0..MAX_FIELD_SIZE) { "policy store field is oversized" }
            require(buffer.remaining() >= size) { "policy store field is truncated" }
            return ByteArray(size).also(buffer::get)
        }
    }
}
