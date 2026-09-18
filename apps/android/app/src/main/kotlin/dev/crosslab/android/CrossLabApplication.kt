package dev.crosslab.android

import android.app.Application
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Build
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import dev.crosslab.android.features.devices.MobileRuntimePort
import dev.crosslab.android.features.devices.RuntimeController

class CrossLabApplication : Application(), DefaultLifecycleObserver {
    lateinit var runtimeController: RuntimeController
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
        runtimeController = RuntimeController(MobileRuntimePort())
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

    override fun onStart(owner: LifecycleOwner) {
        runtimeController.onForeground()
    }

    override fun onStop(owner: LifecycleOwner) {
        runtimeController.onBackground()
    }

    override fun onTerminate() {
        connectivityManager.unregisterNetworkCallback(networkCallback)
        ProcessLifecycleOwner.get().lifecycle.removeObserver(this)
        runtimeController.shutdown()
        super.onTerminate()
    }
}
