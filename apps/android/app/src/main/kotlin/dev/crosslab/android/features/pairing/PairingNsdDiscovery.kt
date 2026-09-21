package dev.crosslab.android.features.pairing

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Handler
import android.os.Looper
import java.net.InetAddress
import java.nio.charset.StandardCharsets
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobilePairingBootstrap

internal const val DEFAULT_PAIRING_DISCOVERY_TIMEOUT_MS = 12_000L

internal data class PairingResolvedRoute(
    val host: InetAddress,
    val port: Int,
)

internal enum class PairingDiscoveryFailure {
    INVALID_INVITATION,
    START_FAILED,
    RESOLVE_FAILED,
    PROFILE_MISMATCH,
    TIMEOUT,
}

internal class AndroidPairingDiscovery(
    context: Context,
    private val timeoutMs: Long = DEFAULT_PAIRING_DISCOVERY_TIMEOUT_MS,
) {
    private val appContext = context.applicationContext
    private val nsdManager = appContext.getSystemService(NsdManager::class.java)
    private val wifiManager = appContext.getSystemService(WifiManager::class.java)
    private val handler = Handler(Looper.getMainLooper())

    fun start(
        bootstrap: MobilePairingBootstrap,
        onResolved: (PairingResolvedRoute) -> Unit,
        onFailure: (PairingDiscoveryFailure) -> Unit,
    ): AutoCloseable {
        val summary =
            runCatching { bootstrap.summary() }
                .getOrElse {
                    onFailure(PairingDiscoveryFailure.INVALID_INVITATION)
                    return AutoCloseable {}
                }
        val serviceType =
            androidNsdServiceType(summary.discoveryServiceType)
                ?: run {
                    onFailure(PairingDiscoveryFailure.INVALID_INVITATION)
                    return AutoCloseable {}
                }

        return DiscoverySession(
            nsdManager = nsdManager,
            wifiManager = wifiManager,
            handler = handler,
            expectedInstance = summary.discoveryInstance,
            serviceType = serviceType,
            timeoutMs = timeoutMs,
            onResolved = onResolved,
            onFailure = onFailure,
        ).also(DiscoverySession::start)
    }
}

private class DiscoverySession(
    private val nsdManager: NsdManager,
    wifiManager: WifiManager?,
    private val handler: Handler,
    private val expectedInstance: String,
    private val serviceType: String,
    private val timeoutMs: Long,
    private val onResolved: (PairingResolvedRoute) -> Unit,
    private val onFailure: (PairingDiscoveryFailure) -> Unit,
) : NsdManager.DiscoveryListener, AutoCloseable {
    private val finished = AtomicBoolean(false)
    private val resolving = AtomicBoolean(false)
    private val multicastLock =
        wifiManager?.createMulticastLock("crosslab-pairing-discovery")?.apply {
            setReferenceCounted(false)
        }
    private val timeout = Runnable { fail(PairingDiscoveryFailure.TIMEOUT) }

    @Volatile
    private var discoveryStarted = false

    fun start() {
        if (finished.get()) return

        try {
            multicastLock?.acquire()
            nsdManager.discoverServices(serviceType, NsdManager.PROTOCOL_DNS_SD, this)
            handler.postDelayed(timeout, timeoutMs)
        } catch (_: RuntimeException) {
            fail(PairingDiscoveryFailure.START_FAILED)
        }
    }

    override fun onDiscoveryStarted(regType: String) {
        discoveryStarted = true
    }

    override fun onServiceFound(serviceInfo: NsdServiceInfo) {
        if (finished.get() ||
            serviceInfo.serviceName != expectedInstance ||
            !resolving.compareAndSet(false, true)
        ) {
            return
        }

        resolve(serviceInfo)
    }

    override fun onServiceLost(serviceInfo: NsdServiceInfo) {
        if (serviceInfo.serviceName == expectedInstance) {
            resolving.set(false)
        }
    }

    override fun onStartDiscoveryFailed(
        serviceType: String,
        errorCode: Int,
    ) {
        fail(PairingDiscoveryFailure.START_FAILED)
    }

    override fun onStopDiscoveryFailed(
        serviceType: String,
        errorCode: Int,
    ) {
        if (!finished.get()) {
            fail(PairingDiscoveryFailure.START_FAILED)
        }
    }

    override fun onDiscoveryStopped(serviceType: String) {
        if (!finished.get()) {
            fail(PairingDiscoveryFailure.START_FAILED)
        }
    }

    @Suppress("DEPRECATION")
    private fun resolve(serviceInfo: NsdServiceInfo) {
        try {
            nsdManager.resolveService(
                serviceInfo,
                object : NsdManager.ResolveListener {
                    override fun onResolveFailed(
                        serviceInfo: NsdServiceInfo,
                        errorCode: Int,
                    ) {
                        resolving.set(false)
                        if (!finished.get()) {
                            fail(PairingDiscoveryFailure.RESOLVE_FAILED)
                        }
                    }

                    override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                        val host = serviceInfo.host
                        val port = serviceInfo.port
                        if (host == null ||
                            port !in 1..65535 ||
                            !hasExactPairingTxtProfile(serviceInfo.attributes)
                        ) {
                            fail(PairingDiscoveryFailure.PROFILE_MISMATCH)
                            return
                        }

                        complete(PairingResolvedRoute(host = host, port = port))
                    }
                },
            )
        } catch (_: RuntimeException) {
            fail(PairingDiscoveryFailure.RESOLVE_FAILED)
        }
    }

    private fun complete(route: PairingResolvedRoute) {
        if (!finished.compareAndSet(false, true)) return
        cleanup()
        onResolved(route)
    }

    private fun fail(reason: PairingDiscoveryFailure) {
        if (!finished.compareAndSet(false, true)) return
        cleanup()
        onFailure(reason)
    }

    override fun close() {
        if (!finished.compareAndSet(false, true)) return
        cleanup()
    }

    private fun cleanup() {
        handler.removeCallbacks(timeout)
        if (discoveryStarted) {
            runCatching { nsdManager.stopServiceDiscovery(this) }
        }
        if (multicastLock?.isHeld == true) {
            multicastLock.release()
        }
    }
}

internal fun androidNsdServiceType(fullType: String): String? {
    val suffix = "local."
    if (!fullType.endsWith(suffix) || fullType.length <= suffix.length) return null
    return fullType.removeSuffix(suffix)
}

internal fun hasExactPairingTxtProfile(attributes: Map<String, ByteArray>): Boolean {
    if (attributes.size != 1) return false
    val version = attributes["v"] ?: return false
    return version.contentEquals("1".toByteArray(StandardCharsets.US_ASCII))
}
