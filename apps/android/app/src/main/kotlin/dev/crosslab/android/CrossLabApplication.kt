package dev.crosslab.android

import android.app.Application
import android.content.pm.ApplicationInfo
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Build
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import dev.crosslab.android.features.devices.MobileRuntimePort
import dev.crosslab.android.features.identity.AndroidEd25519Signer
import dev.crosslab.android.features.identity.AndroidIdentityStore
import dev.crosslab.android.features.identity.AndroidProductIdentityRepository
import dev.crosslab.android.features.identity.AndroidSigningSlot
import dev.crosslab.android.features.devices.RuntimeController
import dev.crosslab.android.features.pairing.PairingJoinerController

class CrossLabApplication : Application(), DefaultLifecycleObserver {
    lateinit var runtimeController: RuntimeController
        private set

    lateinit var identityStore: AndroidIdentityStore
        private set

    lateinit var localDeviceSigner: AndroidEd25519Signer
        private set

    lateinit var identityRepository: AndroidProductIdentityRepository
        private set

    lateinit var pairingController: PairingJoinerController
        private set

    private lateinit var connectivityManager: ConnectivityManager

    private val networkCallback =
        object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                runtimeController.onNetworkAvailable()
            }

            override fun onLost(network: Network) {
                runtimeController.onNetworkLost()
            }
        }

    override fun onCreate() {
        super<Application>.onCreate()
        runtimeController = RuntimeController(MobileRuntimePort(developmentProvisioningPath()))
        identityStore = AndroidIdentityStore(this)
        localDeviceSigner = AndroidEd25519Signer(this, AndroidSigningSlot.LOCAL_DEVICE)
        identityRepository =
            AndroidProductIdentityRepository(
                store = identityStore,
                rootSigner = AndroidEd25519Signer(this, AndroidSigningSlot.OWNER_ROOT),
                deviceSigningSigner =
                    AndroidEd25519Signer(this, AndroidSigningSlot.DEVICE_SIGNING),
                localDeviceSigner = localDeviceSigner,
            )
        pairingController = PairingJoinerController(this, identityRepository)
        ProcessLifecycleOwner.get().lifecycle.addObserver(this)

        connectivityManager = getSystemService(ConnectivityManager::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            connectivityManager.registerDefaultNetworkCallback(networkCallback)
        } else {
            val request =
                NetworkRequest.Builder()
                    .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
                    .build()
            connectivityManager.registerNetworkCallback(request, networkCallback)
        }
    }

    private fun developmentProvisioningPath(): String? {
        if (!BuildConfig.CROSSLAB_DEVELOPMENT_PROVISIONING ||
            applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE == 0
        ) {
            return null
        }

        val provisioning = filesDir.resolve("crosslab-development-provisioning.json")
        return provisioning.takeIf { it.isFile }?.absolutePath
    }

    override fun onStart(owner: LifecycleOwner) {
        runtimeController.onForeground()
    }

    override fun onStop(owner: LifecycleOwner) {
        runtimeController.onBackground()
    }

    override fun onTerminate() {
        connectivityManager.unregisterNetworkCallback(networkCallback)
        ProcessLifecycleOwner.get().lifecycle.removeObserver(this)
        pairingController.close()
        runtimeController.shutdown()
        super.onTerminate()
    }
}
