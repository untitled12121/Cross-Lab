package dev.crosslab.android.features.devices

import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobileRuntime

class MobileRuntimePort(
    private val developmentProvisioningPath: String? = null,
) : RuntimePort {
    override val peerControlAvailable: Boolean = developmentProvisioningPath != null

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

    override fun disconnectPeer() {
        runtime.disconnectPeer()
    }

    override fun reconnectPeer() {
        runtime.reconnectPeer()
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

