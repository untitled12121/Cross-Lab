package dev.crosslab.android.features.devices

import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobileConnectivityState
import uniffi.crosslab_mobile_ffi.MobileNetworkClass
import uniffi.crosslab_mobile_ffi.MobileRuntime
import uniffi.crosslab_mobile_ffi.MobileRuntimeSnapshot
import uniffi.crosslab_mobile_ffi.MobileSessionState
import uniffi.crosslab_mobile_ffi.MobileTransportSecurity
import uniffi.crosslab_mobile_ffi.MobileTrustState

class MobileRuntimePort(
    private val developmentProvisioningPath: String? = null,
) : RuntimePort {
    private val runtime = MobileRuntime()
    private var developmentConfigured = false
    private val listeners = CopyOnWriteArraySet<(RuntimeSnapshot) -> Unit>()
    private val closed = AtomicBoolean(false)
    private val events =
        Executors.newSingleThreadExecutor { task ->
            Thread(task, "crosslab-mobile-runtime-events").apply {
                isDaemon = true
            }
        }

    init {
        events.execute(::eventLoop)
    }

    override fun start() {
        runtime.start()
        if (developmentProvisioningPath != null && !developmentConfigured) {
            try {
                runtime.configureDevelopmentClient(developmentProvisioningPath)
                developmentConfigured = true
            } catch (error: RuntimeException) {
                runCatching { runtime.stop() }
                throw error
            }
        }
    }

    override fun stop() {
        runtime.stop()
        developmentConfigured = false
    }

    override fun networkLost() {
        runtime.networkLost()
    }

    override fun networkAvailable() {
        runtime.networkAvailable()
    }

    override fun snapshot(): RuntimeSnapshot = runtime.snapshot().toRuntimeSnapshot()

    override fun observeSnapshots(listener: (RuntimeSnapshot) -> Unit): AutoCloseable {
        listeners += listener
        listener(snapshot())
        return AutoCloseable { listeners -= listener }
    }

    override fun shutdown() {
        if (!closed.compareAndSet(false, true)) return

        runCatching { runtime.stop() }
        events.shutdownNow()
    }

    private fun eventLoop() {
        while (!closed.get()) {
            val event =
                try {
                    runtime.waitEvent(60_000uL)
                } catch (_: Exception) {
                    if (!closed.get()) continue
                    return
                }
            val snapshot = event?.snapshot?.toRuntimeSnapshot() ?: continue
            listeners.forEach { it(snapshot) }
        }
    }
}

private fun MobileRuntimeSnapshot.toRuntimeSnapshot(): RuntimeSnapshot =
    RuntimeSnapshot(
        peerDeviceId = peerDeviceId,
        trust =
            when (trust) {
                MobileTrustState.UNAVAILABLE -> RuntimeTrust.UNAVAILABLE
                MobileTrustState.PENDING -> RuntimeTrust.PENDING
                MobileTrustState.TRUSTED -> RuntimeTrust.TRUSTED
                MobileTrustState.REVOKED -> RuntimeTrust.REVOKED
            },
        connectivity =
            when (connectivity) {
                MobileConnectivityState.CONNECTED -> RuntimeConnectivity.CONNECTED
                MobileConnectivityState.DISCONNECTED -> RuntimeConnectivity.DISCONNECTED
            },
        session =
            when (session) {
                MobileSessionState.UNAVAILABLE -> RuntimeSession.UNAVAILABLE
                MobileSessionState.CREATED -> RuntimeSession.CREATED
                MobileSessionState.AUTHENTICATING -> RuntimeSession.AUTHENTICATING
                MobileSessionState.ACTIVE -> RuntimeSession.ACTIVE
                MobileSessionState.CLOSING -> RuntimeSession.CLOSING
                MobileSessionState.CLOSED -> RuntimeSession.CLOSED
                MobileSessionState.REVOKED -> RuntimeSession.REVOKED
            },
        protocol =
            protocol?.let {
                RuntimeProtocolVersion(
                    major = it.major.toInt(),
                    minor = it.minor.toInt(),
                )
            },
        network =
            when (network) {
                MobileNetworkClass.UNAVAILABLE -> RuntimeNetwork.UNAVAILABLE
                MobileNetworkClass.LOCAL -> RuntimeNetwork.LOCAL
                MobileNetworkClass.TRUSTED -> RuntimeNetwork.TRUSTED
                MobileNetworkClass.REMOTE -> RuntimeNetwork.REMOTE
            },
        security =
            when (transportSecurity) {
                MobileTransportSecurity.UNAVAILABLE -> RuntimeSecurity.UNAVAILABLE
                MobileTransportSecurity.TEST_ONLY -> RuntimeSecurity.TEST_ONLY
                MobileTransportSecurity.AUTHENTICATED -> RuntimeSecurity.AUTHENTICATED
            },
        metered = metered,
        capabilityCount = capabilityCount.coerceAtMost(Int.MAX_VALUE.toULong()).toInt(),
    )
