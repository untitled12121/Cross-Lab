package dev.crosslab.android.features.devices

import android.content.Context
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import dev.crosslab.android.features.identity.AndroidProductIdentityRepository
import uniffi.crosslab_mobile_ffi.MobilePresenceDiscovery
import uniffi.crosslab_mobile_ffi.MobilePresencePhase
import uniffi.crosslab_mobile_ffi.MobilePresenceSnapshot
import uniffi.crosslab_mobile_ffi.MobileTrustedPresenceAgent

class ProductPresencePort(
    context: Context,
    private val identityRepository: AndroidProductIdentityRepository,
) : RuntimePort {
    override val peerControlAvailable: Boolean = true

    private val discovery = AndroidTrustedSessionDiscovery(context)
    private val listeners = CopyOnWriteArraySet<(RuntimeSnapshot) -> Unit>()
    private val events =
        Executors.newSingleThreadExecutor { task ->
            Thread(task, "crosslab-presence-events").apply { isDaemon = true }
        }
    private val closed = AtomicBoolean(false)
    private val lock = Any()

    @Volatile
    private var current = RuntimeSnapshot.disconnected()

    private var agent: MobileTrustedPresenceAgent? = null
    private var discoveryInfo: MobilePresenceDiscovery? = null
    private var discoveryHandle: AutoCloseable? = null
    private var generation: Long = 0

    override fun start() {
        synchronized(lock) {
            if (closed.get() || agent != null) return
            startAgentLocked()
        }
    }

    override fun stop() {
        synchronized(lock) {
            stopAgentLocked()
            publishLocked(RuntimeSnapshot.disconnected())
        }
    }

    override fun networkLost() {
        synchronized(lock) {
            discoveryHandle?.close()
            discoveryHandle = null
            agent?.networkLost()
            publishAgentSnapshotLocked()
        }
    }

    override fun networkAvailable() {
        synchronized(lock) {
            val active = agent
            if (active == null) {
                startAgentLocked()
                return
            }
            active.networkAvailable()
            ensureDiscoveryLocked(active)
            publishAgentSnapshotLocked()
        }
    }

    override fun disconnectPeer() {
        synchronized(lock) {
            val active = agent ?: return
            discoveryHandle?.close()
            discoveryHandle = null
            active.disconnect()
            publishAgentSnapshotLocked()
        }
    }

    override fun reconnectPeer() {
        synchronized(lock) {
            val active = agent
            if (active == null) {
                startAgentLocked()
                return
            }
            active.reconnect()
            ensureDiscoveryLocked(active)
            publishAgentSnapshotLocked()
        }
    }

    override fun snapshot(): RuntimeSnapshot = current

    override fun observeSnapshots(listener: (RuntimeSnapshot) -> Unit): AutoCloseable {
        listeners += listener
        listener(current)
        return AutoCloseable { listeners -= listener }
    }

    override fun shutdown() {
        if (!closed.compareAndSet(false, true)) return
        synchronized(lock) {
            stopAgentLocked()
            publishLocked(RuntimeSnapshot.disconnected())
        }
        events.shutdownNow()
        listeners.clear()
    }

    private fun startAgentLocked() {
        val identity =
            runCatching { identityRepository.load() }
                .getOrNull()
                ?: run {
                    publishLocked(RuntimeSnapshot.disconnected())
                    return
                }
        if (identity.trustedPeerCount == 0uL) {
            publishLocked(RuntimeSnapshot.disconnected())
            return
        }

        val active =
            runCatching {
                MobileTrustedPresenceAgent(
                    identityPayload = identity.payload,
                    localDeviceSigner = identityRepository.localDeviceSigner,
                )
            }.getOrElse {
                publishLocked(
                    RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
                )
                return
            }

        val info =
            runCatching { active.discovery() }
                .getOrElse {
                    runCatching { active.close() }
                    publishLocked(
                        RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
                    )
                    return
                }

        generation += 1
        val token = generation
        agent = active
        discoveryInfo = info
        ensureDiscoveryLocked(active)
        publishLocked(active.snapshot().toRuntimeSnapshot())
        events.execute { eventLoop(active, token) }
    }

    private fun stopAgentLocked() {
        generation += 1
        discoveryHandle?.close()
        discoveryHandle = null
        discoveryInfo = null
        val active = agent
        agent = null
        if (active != null) {
            runCatching { active.close() }
        }
    }

    private fun ensureDiscoveryLocked(active: MobileTrustedPresenceAgent) {
        if (discoveryHandle != null) return
        val info = discoveryInfo ?: return

        discoveryHandle =
            discovery.start(
                profile = info.profile,
                port = info.listenPort.toInt(),
                onCandidate = { route ->
                    synchronized(lock) {
                        if (agent !== active) return@start
                        runCatching {
                            active.candidateAvailable(
                                instance = route.instance,
                                address = route.addressBytes,
                                port = route.port,
                                scopeId = route.scopeId,
                            )
                        }
                    }
                },
                onLost = { instance ->
                    synchronized(lock) {
                        if (agent !== active) return@start
                        runCatching { active.candidateLost(instance) }
                    }
                },
                onFailure = {
                    synchronized(lock) {
                        if (agent !== active) return@start
                        discoveryHandle = null
                        publishLocked(
                            current.copy(presence = RuntimePresence.FAILED),
                        )
                    }
                },
            )
    }

    private fun eventLoop(
        active: MobileTrustedPresenceAgent,
        token: Long,
    ) {
        while (!closed.get()) {
            val event =
                try {
                    active.waitEvent(60_000uL)
                } catch (_: Exception) {
                    return
                } ?: continue

            synchronized(lock) {
                if (generation != token || agent !== active) return
                publishLocked(event.toRuntimeSnapshot())
            }
        }
    }

    private fun publishAgentSnapshotLocked() {
        val active = agent ?: return
        runCatching { active.snapshot() }
            .onSuccess { publishLocked(it.toRuntimeSnapshot()) }
    }

    private fun publishLocked(snapshot: RuntimeSnapshot) {
        current = snapshot
        listeners.forEach { it(snapshot) }
    }
}

private fun MobilePresenceSnapshot.toRuntimeSnapshot(): RuntimeSnapshot {
    val presence =
        when (phase) {
            MobilePresencePhase.DISCOVERING -> RuntimePresence.DISCOVERING
            MobilePresencePhase.CONNECTING -> RuntimePresence.CONNECTING
            MobilePresencePhase.ONLINE -> RuntimePresence.ONLINE
            MobilePresencePhase.RECONNECTING -> RuntimePresence.RECONNECTING
            MobilePresencePhase.PAUSED -> RuntimePresence.PAUSED
            MobilePresencePhase.FAILED -> RuntimePresence.FAILED
        }
    return runtime?.toRuntimeSnapshot(presence)
        ?: RuntimeSnapshot.disconnected().copy(presence = presence)
}
