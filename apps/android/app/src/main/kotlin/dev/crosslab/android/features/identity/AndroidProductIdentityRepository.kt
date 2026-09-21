package dev.crosslab.android.features.identity

import uniffi.crosslab_mobile_ffi.MobileProductIdentity
import uniffi.crosslab_mobile_ffi.MobileProductPairingCommit
import uniffi.crosslab_mobile_ffi.MobileProductPairingJoinerCompletion
import uniffi.crosslab_mobile_ffi.productIdentityApplyPairingCommit
import uniffi.crosslab_mobile_ffi.productIdentityFromJoinerCompletion
import uniffi.crosslab_mobile_ffi.productIdentityLoad
import uniffi.crosslab_mobile_ffi.productIdentityLoadOrCreate

class AndroidProductIdentityRepository(
    private val store: AndroidIdentityStore,
    private val rootSigner: AndroidEd25519Signer,
    private val deviceSigningSigner: AndroidEd25519Signer,
    val localDeviceSigner: AndroidEd25519Signer,
) {
    @Synchronized
    fun load(): MobileProductIdentity? {
        val payload = store.loadPayload() ?: return null
        return productIdentityLoad(payload, localDeviceSigner)
    }

    @Synchronized
    fun loadOrBootstrapOwner(): MobileProductIdentity {
        store.loadPayload()?.let { payload ->
            return productIdentityLoad(payload, localDeviceSigner)
        }

        val identity =
            productIdentityLoadOrCreate(
                currentPayload = null,
                rootSigner = rootSigner,
                deviceSigningSigner = deviceSigningSigner,
                localDeviceSigner = localDeviceSigner,
            )
        store.commitPayload(identity.payload)
        return identity
    }

    @Synchronized
    fun commitInviterPairing(commit: MobileProductPairingCommit): MobileProductIdentity {
        if (!rootSigner.exists || !deviceSigningSigner.exists) {
            throw IdentityStoreUnavailable("device does not hold pairing authority")
        }
        val payload =
            store.loadPayload()
                ?: throw IdentityStoreUnavailable("product identity is unavailable")

        val next =
            productIdentityApplyPairingCommit(
                currentPayload = payload,
                commit = commit,
                rootSigner = rootSigner,
                deviceSigningSigner = deviceSigningSigner,
                localDeviceSigner = localDeviceSigner,
            )
        store.commitPayload(next.payload)
        return next
    }

    @Synchronized
    fun commitJoinerPairing(
        completion: MobileProductPairingJoinerCompletion,
    ): MobileProductIdentity {
        if (store.loadPayload() != null) {
            throw IdentityStoreUnavailable(
                "existing product identity cannot be replaced by pairing",
            )
        }

        val next =
            productIdentityFromJoinerCompletion(
                completion = completion,
                localDeviceSigner = localDeviceSigner,
            )
        store.commitPayload(next.payload)
        return next
    }
}
