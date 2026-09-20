package dev.crosslab.android.features.identity

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.nio.ByteBuffer
import java.security.KeyFactory
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.Signature
import java.security.spec.PKCS8EncodedKeySpec
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class AndroidEd25519Signer(
    context: Context,
) {
    private val keyFile =
        AtomicFile(
            context.noBackupFilesDir
                .resolve("crosslab/identity/device-signer-v1.bin")
                .apply { parentFile?.mkdirs() },
        )
    private val keyStore =
        KeyStore.getInstance(ANDROID_KEY_STORE).apply {
            load(null)
        }

    val available: Boolean
        get() = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU

    @Synchronized
    fun ensureCreated(): ByteArray {
        requireSupported()

        if (keyFile.baseFile.isFile) {
            return SignerBundle.decode(keyFile.readFully()).publicKeyRaw
        }

        val pair = KeyPairGenerator.getInstance(ED25519).generateKeyPair()
        val privateBytes =
            pair.private.encoded ?: throw IdentityStoreUnavailable("Ed25519 private key is not encodable")
        val publicRaw = extractRawPublicKey(pair.public.encoded)
        val encrypted = encrypt(privateBytes)
        privateBytes.fill(0)

        val bundle = SignerBundle(publicRaw, encrypted.iv, encrypted.ciphertext)
        val output = keyFile.startWrite()
        try {
            output.write(bundle.encode())
            output.fd.sync()
            keyFile.finishWrite(output)
        } catch (error: Throwable) {
            keyFile.failWrite(output)
            throw error
        }

        return publicRaw
    }

    @Synchronized
    fun publicKey(): ByteArray = ensureCreated()

    @Synchronized
    fun sign(message: ByteArray): ByteArray {
        requireSupported()
        val bundle = SignerBundle.decode(keyFile.readFully())
        val privateBytes = decrypt(bundle.iv, bundle.ciphertext)
        try {
            val privateKey =
                KeyFactory.getInstance(ED25519)
                    .generatePrivate(PKCS8EncodedKeySpec(privateBytes))
            return Signature.getInstance(ED25519).run {
                initSign(privateKey)
                update(message)
                sign()
            }
        } finally {
            privateBytes.fill(0)
        }
    }

    @Synchronized
    fun wipe() {
        keyFile.delete()
        if (keyStore.containsAlias(WRAP_ALIAS)) {
            keyStore.deleteEntry(WRAP_ALIAS)
        }
    }

    private fun encrypt(plaintext: ByteArray): EncryptedValue {
        val cipher = Cipher.getInstance(AES_GCM)
        cipher.init(Cipher.ENCRYPT_MODE, wrappingKey())
        return EncryptedValue(
            iv = cipher.iv,
            ciphertext = cipher.doFinal(plaintext),
        )
    }

    private fun decrypt(
        iv: ByteArray,
        ciphertext: ByteArray,
    ): ByteArray =
        Cipher.getInstance(AES_GCM).run {
            init(Cipher.DECRYPT_MODE, wrappingKey(), GCMParameterSpec(GCM_TAG_BITS, iv))
            doFinal(ciphertext)
        }

    private fun wrappingKey(): SecretKey {
        (keyStore.getKey(WRAP_ALIAS, null) as? SecretKey)?.let { return it }

        val generator =
            KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_AES,
                ANDROID_KEY_STORE,
            )
        generator.init(
            KeyGenParameterSpec.Builder(
                WRAP_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .setUserAuthenticationRequired(false)
                .build(),
        )
        return generator.generateKey()
    }

    private fun requireSupported() {
        check(available) {
            "production Ed25519 identity requires Android 13/API 33 or newer on this build"
        }
    }

    private fun extractRawPublicKey(encoded: ByteArray): ByteArray {
        require(encoded.size >= ED25519_SPKI_PREFIX.size + 32) {
            "Ed25519 public key encoding is malformed"
        }
        require(
            encoded.copyOfRange(0, ED25519_SPKI_PREFIX.size)
                .contentEquals(ED25519_SPKI_PREFIX),
        ) {
            "Ed25519 public key encoding is unsupported"
        }
        return encoded.copyOfRange(ED25519_SPKI_PREFIX.size, ED25519_SPKI_PREFIX.size + 32)
    }

    private data class EncryptedValue(
        val iv: ByteArray,
        val ciphertext: ByteArray,
    )

    companion object {
        private const val ANDROID_KEY_STORE = "AndroidKeyStore"
        private const val WRAP_ALIAS = "crosslab.identity.ed25519-wrap.v1"
        private const val ED25519 = "Ed25519"
        private const val AES_GCM = "AES/GCM/NoPadding"
        private const val GCM_TAG_BITS = 128
        private val ED25519_SPKI_PREFIX =
            byteArrayOf(
                0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
            )
    }
}

internal data class SignerBundle(
    val publicKeyRaw: ByteArray,
    val iv: ByteArray,
    val ciphertext: ByteArray,
) {
    fun encode(): ByteArray {
        val total =
            MAGIC.size +
                Int.SIZE_BYTES + publicKeyRaw.size +
                Int.SIZE_BYTES + iv.size +
                Int.SIZE_BYTES + ciphertext.size
        return ByteBuffer.allocate(total).apply {
            put(MAGIC)
            putField(publicKeyRaw)
            putField(iv)
            putField(ciphertext)
        }.array()
    }

    companion object {
        private val MAGIC = byteArrayOf(0x43, 0x4c, 0x53, 0x4b, 0x01)
        private const val MAX_FIELD = 16 * 1024

        fun decode(bytes: ByteArray): SignerBundle {
            val buffer = ByteBuffer.wrap(bytes)
            val magic = ByteArray(MAGIC.size)
            require(buffer.remaining() >= magic.size)
            buffer.get(magic)
            require(magic.contentEquals(MAGIC))

            val publicKey = buffer.getField()
            require(publicKey.size == 32)
            val iv = buffer.getField()
            val ciphertext = buffer.getField()
            require(!buffer.hasRemaining())
            return SignerBundle(publicKey, iv, ciphertext)
        }

        private fun ByteBuffer.putField(value: ByteArray) {
            require(value.size <= MAX_FIELD)
            putInt(value.size)
            put(value)
        }

        private fun ByteBuffer.getField(): ByteArray {
            require(remaining() >= Int.SIZE_BYTES)
            val size = int
            require(size in 0..MAX_FIELD)
            require(remaining() >= size)
            return ByteArray(size).also(::get)
        }
    }
}
