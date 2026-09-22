package dev.crosslab.android.features.devices

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Handler
import android.os.Looper
import java.net.Inet6Address
import java.net.InetAddress
import java.nio.charset.StandardCharsets
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobileTrustedSessionDiscoveryProfile
import uniffi.crosslab_mobile_ffi.trustedSessionDiscoveryInstanceValid

internal data class TrustedSessionRoute(
    val instance: String,
    val host: InetAddress,
    val port: Int,
) {
    val addressBytes: ByteArray
        get() = host.address

    val scopeId: Int
        get() = (host as? Inet6Address)?.scopeId ?: 0
}

internal enum class TrustedSessionDiscoveryFailure {
    INVALID_PROFILE,
    ADVERTISEMENT_FAILED,
    DISCOVERY_FAILED,
}

internal class AndroidTrustedSessionDiscovery(
    context: Context,
) {
    private val appContext = context.applicationContext
    private val nsdManager = appContext.getSystemService(NsdManager::class.java)
    private val wifiManager = appContext.getSystemService(WifiManager::class.java)
    private val handler = Handler(Looper.getMainLooper())

    fun start(
        profile: MobileTrustedSessionDiscoveryProfile,
        port: Int,
        onCandidate: (TrustedSessionRoute) -> Unit,
        onLost: (String) -> Unit,
        onFailure: (TrustedSessionDiscoveryFailure) -> Unit,
    ): AutoCloseable {
        val serviceType =
            androidNsdServiceType(profile.serviceType)
                ?: run {
                    onFailure(TrustedSessionDiscoveryFailure.INVALID_PROFILE)
                    return AutoCloseable {}
                }
        if (port !in 1..65535 ||
            !trustedSessionDiscoveryInstanceValid(profile.instance) ||
            profile.maxCandidates == 0u
        ) {
            onFailure(TrustedSessionDiscoveryFailure.INVALID_PROFILE)
            return AutoCloseable {}
        }

        return TrustedSessionDiscoverySession(
            nsdManager = nsdManager,
            wifiManager = wifiManager,
            handler = handler,
            profile = profile,
            serviceType = serviceType,
            port = port,
            onCandidate = onCandidate,
            onLost = onLost,
            onFailure = onFailure,
        ).also(TrustedSessionDiscoverySession::start)
    }
}

private class TrustedSessionDiscoverySession(
    private val nsdManager: NsdManager,
    wifiManager: WifiManager?,
    private val handler: Handler,
    private val profile: MobileTrustedSessionDiscoveryProfile,
    private val serviceType: String,
    private val port: Int,
    private val onCandidate: (TrustedSessionRoute) -> Unit,
    private val onLost: (String) -> Unit,
    private val onFailure: (TrustedSessionDiscoveryFailure) -> Unit,
) : NsdManager.DiscoveryListener, NsdManager.RegistrationListener, AutoCloseable {
    private val closed = AtomicBoolean(false)
    private val candidates = LinkedHashMap<String, TrustedSessionRoute>()
    private val resolving = LinkedHashSet<String>()
    private val multicastLock =
        wifiManager?.createMulticastLock("crosslab-session-discovery")?.apply {
            setReferenceCounted(false)
        }

    @Volatile
    private var discoveryStarted = false

    @Volatile
    private var serviceRegistered = false

    fun start() {
        if (closed.get()) return

        try {
            multicastLock?.acquire()
            nsdManager.registerService(localServiceInfo(), NsdManager.PROTOCOL_DNS_SD, this)
            nsdManager.discoverServices(serviceType, NsdManager.PROTOCOL_DNS_SD, this)
        } catch (_: RuntimeException) {
            fail(TrustedSessionDiscoveryFailure.DISCOVERY_FAILED)
        }
    }

    override fun onServiceRegistered(serviceInfo: NsdServiceInfo) {
        if (closed.get()) return
        if (serviceInfo.serviceName != profile.instance) {
            fail(TrustedSessionDiscoveryFailure.ADVERTISEMENT_FAILED)
            return
        }
        serviceRegistered = true
    }

    override fun onRegistrationFailed(
        serviceInfo: NsdServiceInfo,
        errorCode: Int,
    ) {
        fail(TrustedSessionDiscoveryFailure.ADVERTISEMENT_FAILED)
    }

    override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) {
        serviceRegistered = false
    }

    override fun onUnregistrationFailed(
        serviceInfo: NsdServiceInfo,
        errorCode: Int,
    ) {
        serviceRegistered = false
    }

    override fun onDiscoveryStarted(regType: String) {
        discoveryStarted = true
    }

    override fun onServiceFound(serviceInfo: NsdServiceInfo) {
        val instance = serviceInfo.serviceName
        if (closed.get() ||
            instance == profile.instance ||
            !trustedSessionDiscoveryInstanceValid(instance)
        ) {
            return
        }

        synchronized(candidates) {
            if (instance in candidates || instance in resolving) return
            if (candidates.size + resolving.size >= profile.maxCandidates.toInt()) return
            resolving += instance
        }
        resolve(serviceInfo, instance)
    }

    override fun onServiceLost(serviceInfo: NsdServiceInfo) {
        val instance = serviceInfo.serviceName
        val removed =
            synchronized(candidates) {
                resolving.remove(instance)
                candidates.remove(instance) != null
            }
        if (removed) {
            onLost(instance)
        }
    }

    override fun onStartDiscoveryFailed(
        serviceType: String,
        errorCode: Int,
    ) {
        fail(TrustedSessionDiscoveryFailure.DISCOVERY_FAILED)
    }

    override fun onStopDiscoveryFailed(
        serviceType: String,
        errorCode: Int,
    ) {
        if (!closed.get()) {
            fail(TrustedSessionDiscoveryFailure.DISCOVERY_FAILED)
        }
    }

    override fun onDiscoveryStopped(serviceType: String) {
        discoveryStarted = false
        if (!closed.get()) {
            fail(TrustedSessionDiscoveryFailure.DISCOVERY_FAILED)
        }
    }

    @Suppress("DEPRECATION")
    private fun resolve(
        serviceInfo: NsdServiceInfo,
        instance: String,
    ) {
        try {
            nsdManager.resolveService(
                serviceInfo,
                object : NsdManager.ResolveListener {
                    override fun onResolveFailed(
                        serviceInfo: NsdServiceInfo,
                        errorCode: Int,
                    ) {
                        synchronized(candidates) {
                            resolving.remove(instance)
                        }
                    }

                    override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                        val host = serviceInfo.host
                        val resolvedPort = serviceInfo.port
                        val valid =
                            host != null &&
                                resolvedPort in 1..65535 &&
                                hasExactTrustedSessionTxtProfile(
                                    attributes = serviceInfo.attributes,
                                    key = profile.txtVersionKey,
                                    version = profile.txtVersion,
                                )
                        if (!valid) {
                            synchronized(candidates) {
                                resolving.remove(instance)
                            }
                            return
                        }

                        val route =
                            TrustedSessionRoute(
                                instance = instance,
                                host = requireNotNull(host),
                                port = resolvedPort,
                            )
                        val accepted =
                            synchronized(candidates) {
                                resolving.remove(instance)
                                if (candidates.size >= profile.maxCandidates.toInt()) {
                                    false
                                } else {
                                    candidates[instance] = route
                                    true
                                }
                            }
                        if (accepted && !closed.get()) {
                            onCandidate(route)
                        }
                    }
                },
            )
        } catch (_: RuntimeException) {
            synchronized(candidates) {
                resolving.remove(instance)
            }
        }
    }

    private fun localServiceInfo(): NsdServiceInfo =
        NsdServiceInfo().apply {
            serviceName = profile.instance
            serviceType = this@TrustedSessionDiscoverySession.serviceType
            port = this@TrustedSessionDiscoverySession.port
            setAttribute(
                profile.txtVersionKey,
                profile.txtVersion.toByteArray(StandardCharsets.US_ASCII),
            )
        }

    private fun fail(reason: TrustedSessionDiscoveryFailure) {
        if (!closed.compareAndSet(false, true)) return
        cleanup()
        onFailure(reason)
    }

    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        cleanup()
    }

    private fun cleanup() {
        handler.removeCallbacksAndMessages(null)
        if (discoveryStarted) {
            runCatching { nsdManager.stopServiceDiscovery(this) }
        }
        if (serviceRegistered) {
            runCatching { nsdManager.unregisterService(this) }
        }
        synchronized(candidates) {
            resolving.clear()
            candidates.clear()
        }
        if (multicastLock?.isHeld == true) {
            multicastLock.release()
        }
    }
}

internal fun hasExactTrustedSessionTxtProfile(
    attributes: Map<String, ByteArray>,
    key: String,
    version: String,
): Boolean {
    if (attributes.size != 1) return false
    val value = attributes[key] ?: return false
    return value.contentEquals(version.toByteArray(StandardCharsets.US_ASCII))
}
