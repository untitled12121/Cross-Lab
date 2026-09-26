package dev.crosslab.android.features.devices

import android.content.Context
import android.os.Handler
import android.os.Looper
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import dev.crosslab.android.features.clipboard.AndroidClipboardAdapter
import dev.crosslab.android.features.clipboard.ClipboardPort
import dev.crosslab.android.features.clipboard.ClipboardResult
import dev.crosslab.android.features.clipboard.LocalClipboardRead
import dev.crosslab.android.features.clipboard.LocalClipboardWrite
import dev.crosslab.android.features.identity.AndroidProductIdentityRepository
import dev.crosslab.android.features.permissions.AndroidPolicyStore
import dev.crosslab.android.features.permissions.PolicyStoreUnavailable
import uniffi.crosslab_mobile_ffi.MobileClipboardOutcome
import uniffi.crosslab_mobile_ffi.MobileClipboardPlatformFailure
import uniffi.crosslab_mobile_ffi.MobileClipboardRequest
import uniffi.crosslab_mobile_ffi.MobileClipboardRequestKind
import uniffi.crosslab_mobile_ffi.MobilePermissionEffect
import uniffi.crosslab_mobile_ffi.MobilePermissionSnapshot
import uniffi.crosslab_mobile_ffi.MobilePresenceDiscovery
import uniffi.crosslab_mobile_ffi.MobilePresencePhase
import uniffi.crosslab_mobile_ffi.MobilePresenceSnapshot
import uniffi.crosslab_mobile_ffi.MobileTrustedPresenceAgent
import uniffi.crosslab_mobile_ffi.policyStorePrepareRuleEffect

private const val DISCOVERY_RETRY_MS = 2_000L

class ProductPresencePort(
    context: Context,
    private val identityRepository: AndroidProductIdentityRepository,
    private val policyStore: AndroidPolicyStore,
) : RuntimePort, ClipboardPort {
    override val peerControlAvailable: Boolean = true
    override val clipboardAvailable: Boolean = true

    private val discovery = AndroidTrustedSessionDiscovery(context)
    private val clipboard = AndroidClipboardAdapter(context)
    private val listeners = CopyOnWriteArraySet<(RuntimeSnapshot) -> Unit>()
    private val events =
        Executors.newSingleThreadExecutor { task ->
            Thread(task, "crosslab-presence-events").apply { isDaemon = true }
        }
    private val clipboardEvents =
        Executors.newSingleThreadExecutor { task ->
            Thread(task, "crosslab-clipboard-events").apply { isDaemon = true }
        }
    private val clipboardWorkers =
        Executors.newFixedThreadPool(2) { task ->
            Thread(task, "crosslab-clipboard-worker").apply { isDaemon = true }
        }
    private val closed = AtomicBoolean(false)
    private val lock = Any()
    private val handler = Handler(Looper.getMainLooper())

    @Volatile
    private var current = RuntimeSnapshot.disconnected()

    private var agent: MobileTrustedPresenceAgent? = null
    private var discoveryInfo: MobilePresenceDiscovery? = null
    private var discoveryHandle: AutoCloseable? = null
    private var discoveryRetry: Runnable? = null
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
            cancelDiscoveryRetryLocked()
            discoveryHandle?.close()
            discoveryHandle = null
            agent?.networkLost()
            publishAgentSnapshotLocked()
        }
    }

    override fun networkAvailable() {
        synchronized(lock) {
            cancelDiscoveryRetryLocked()
            val active = agent
            if (active == null) {
                startAgentLocked()
                return
            }
            if (discoveryHandle == null && !rotateDiscoveryLocked(active)) return
            active.networkAvailable()
            ensureDiscoveryLocked(active)
            publishAgentSnapshotLocked()
        }
    }

    override fun disconnectPeer() {
        synchronized(lock) {
            cancelDiscoveryRetryLocked()
            val active = agent ?: return
            discoveryHandle?.close()
            discoveryHandle = null
            active.disconnect()
            publishAgentSnapshotLocked()
        }
    }

    override fun reconnectPeer() {
        synchronized(lock) {
            cancelDiscoveryRetryLocked()
            val active = agent
            if (active == null) {
                startAgentLocked()
                return
            }
            if (!rotateDiscoveryLocked(active)) return
            active.reconnect()
            ensureDiscoveryLocked(active)
            publishAgentSnapshotLocked()
        }
    }

    override fun sendClipboard(onComplete: (ClipboardResult) -> Unit): Boolean {
        val (active, token) =
            synchronized(lock) {
                agent?.let { it to generation }
            } ?: return false

        handler.post {
            if (!isCurrentAgent(active, token)) {
                onComplete(ClipboardResult.CANCELLED)
                return@post
            }

            when (val local = clipboard.readText()) {
                is LocalClipboardRead.Text ->
                    clipboardWorkers.execute {
                        val result =
                            runCatching { active.sendClipboardText(local.value) }
                                .getOrNull()
                        val outcome =
                            result?.outcome()?.toClipboardResult()
                                ?: ClipboardResult.FAILED
                        handler.post {
                            onComplete(
                                if (isCurrentAgent(active, token)) {
                                    outcome
                                } else {
                                    ClipboardResult.CANCELLED
                                },
                            )
                        }
                    }

                LocalClipboardRead.Unavailable -> onComplete(ClipboardResult.UNAVAILABLE)
                LocalClipboardRead.Failed -> onComplete(ClipboardResult.FAILED)
            }
        }
        return true
    }

    override fun fetchClipboard(onComplete: (ClipboardResult) -> Unit): Boolean {
        val (active, token) =
            synchronized(lock) {
                agent?.let { it to generation }
            } ?: return false

        clipboardWorkers.execute {
            val result =
                runCatching { active.fetchClipboardText() }
                    .getOrNull()
            val outcome =
                result?.outcome()?.toClipboardResult()
                    ?: ClipboardResult.FAILED
            if (outcome != ClipboardResult.SUCCESS) {
                handler.post {
                    onComplete(
                        if (isCurrentAgent(active, token)) {
                            outcome
                        } else {
                            ClipboardResult.CANCELLED
                        },
                    )
                }
                return@execute
            }

            val text = result?.let { operation ->
                runCatching { operation.takeText() }.getOrNull()
            }
            handler.post {
                if (!isCurrentAgent(active, token)) {
                    onComplete(ClipboardResult.CANCELLED)
                    return@post
                }
                if (text == null) {
                    onComplete(ClipboardResult.FAILED)
                    return@post
                }
                onComplete(
                    when (clipboard.writeRemoteText(text)) {
                        LocalClipboardWrite.SUCCESS -> ClipboardResult.SUCCESS
                        LocalClipboardWrite.FAILED -> ClipboardResult.FAILED
                    },
                )
            }
        }
        return true
    }

    override fun setPermission(
        capabilityId: String,
        operation: String,
        effect: RuntimePermissionEffect,
    ): Boolean =
        synchronized(lock) {
            val active = agent ?: return@synchronized false
            val peerDeviceId = current.peerDeviceId ?: return@synchronized false
            val permissions = active.permissionSnapshot()
            val stored =
                try {
                    policyStore.load()
                } catch (error: Throwable) {
                    failClosedPolicyLocked(active)
                    throw error
                }
            val storedRevision = stored?.anchorRevision() ?: 0uL
            if (storedRevision != permissions.policyRevision) {
                failClosedPolicyLocked(active)
                throw PolicyStoreUnavailable("active policy revision does not match durable policy")
            }

            val commit =
                policyStorePrepareRuleEffect(
                    currentEnvelope = stored?.envelope,
                    currentAnchor = stored?.anchor,
                    sourceDeviceId = peerDeviceId,
                    capabilityId = capabilityId,
                    operation = operation,
                    effect = effect.toMobilePermissionEffect(),
                ) ?: return@synchronized false

            try {
                policyStore.commit(
                    expectedRevision = permissions.policyRevision,
                    envelope = commit.envelope,
                    anchor = commit.anchor,
                )
            } catch (error: Throwable) {
                val recovered = runCatching { policyStore.load() }
                if (recovered.isSuccess) {
                    val durable = recovered.getOrNull()
                    val durableRevision = durable?.anchorRevision() ?: 0uL
                    if (durableRevision == permissions.policyRevision) {
                        throw error
                    }
                    if (
                        durable != null &&
                            durableRevision == commit.revision &&
                            durable.envelope.contentEquals(commit.envelope) &&
                            durable.anchor.contentEquals(commit.anchor)
                    ) {
                        try {
                            active.replacePolicy(durable.envelope, durable.anchor)
                        } catch (applyError: Throwable) {
                            failClosedPolicyLocked(active)
                            throw applyError
                        }
                        publishAgentSnapshotLocked()
                        return@synchronized true
                    }
                }
                failClosedPolicyLocked(active)
                throw error
            }

            try {
                active.replacePolicy(commit.envelope, commit.anchor)
            } catch (error: Throwable) {
                failClosedPolicyLocked(active)
                throw error
            }
            publishAgentSnapshotLocked()
            true
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
        handler.removeCallbacksAndMessages(null)
        events.shutdownNow()
        clipboardEvents.shutdownNow()
        clipboardWorkers.shutdownNow()
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

        val policy =
            runCatching { policyStore.load() }
                .getOrElse {
                    publishLocked(
                        RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
                    )
                    return
                }

        val active =
            runCatching {
                MobileTrustedPresenceAgent(
                    identityPayload = identity.payload,
                    localDeviceSigner = identityRepository.localDeviceSigner,
                    policyEnvelope = policy?.envelope,
                    policyAnchor = policy?.anchor,
                    clipboardReadAvailable = clipboardAvailable,
                    clipboardWriteAvailable = clipboardAvailable,
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
                    runCatching { active.shutdown() }
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
        publishLocked(
            active.snapshot().toRuntimeSnapshot(
                permissions = active.permissionSnapshot(),
            ),
        )
        events.execute { eventLoop(active, token) }
        clipboardEvents.execute { clipboardEventLoop(active, token) }
    }

    private fun stopAgentLocked() {
        generation += 1
        cancelDiscoveryRetryLocked()
        discoveryHandle?.close()
        discoveryHandle = null
        discoveryInfo = null
        val active = agent
        agent = null
        if (active != null) {
            runCatching { active.shutdown() }
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
                        runCatching { active.networkLost() }
                        publishAgentSnapshotLocked()
                        scheduleDiscoveryRetryLocked(active)
                    }
                },
            )
    }

    private fun rotateDiscoveryLocked(active: MobileTrustedPresenceAgent): Boolean {
        val info =
            runCatching { active.rotateDiscovery() }
                .getOrElse {
                    publishLocked(
                        RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
                    )
                    return false
                }
        discoveryInfo = info
        return true
    }

    private fun scheduleDiscoveryRetryLocked(active: MobileTrustedPresenceAgent) {
        cancelDiscoveryRetryLocked()
        val retry =
            Runnable {
                synchronized(lock) {
                    discoveryRetry = null
                    if (closed.get() || agent !== active || discoveryHandle != null) {
                        return@synchronized
                    }
                    if (!rotateDiscoveryLocked(active)) return@synchronized
                    runCatching { active.networkAvailable() }
                    ensureDiscoveryLocked(active)
                    publishAgentSnapshotLocked()
                }
            }
        discoveryRetry = retry
        handler.postDelayed(retry, DISCOVERY_RETRY_MS)
    }

    private fun cancelDiscoveryRetryLocked() {
        discoveryRetry?.let(handler::removeCallbacks)
        discoveryRetry = null
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
                publishLocked(
                    event.toRuntimeSnapshot(
                        permissions = runCatching { active.permissionSnapshot() }.getOrNull(),
                    ),
                )
            }
        }
    }

    private fun clipboardEventLoop(
        active: MobileTrustedPresenceAgent,
        token: Long,
    ) {
        while (!closed.get()) {
            val request =
                try {
                    active.waitClipboardRequest(60_000uL)
                } catch (_: Exception) {
                    return
                } ?: continue

            if (!isCurrentAgent(active, token)) return
            handler.post {
                handleClipboardRequest(active, token, request)
            }
        }
    }

    private fun handleClipboardRequest(
        active: MobileTrustedPresenceAgent,
        token: Long,
        request: MobileClipboardRequest,
    ) {
        if (!isCurrentAgent(active, token)) return

        val requestId = request.requestId()
        when (request.kind()) {
            MobileClipboardRequestKind.READ -> {
                when (val local = clipboard.readText()) {
                    is LocalClipboardRead.Text ->
                        clipboardWorkers.execute {
                            runCatching {
                                active.completeClipboardRead(requestId, local.value)
                            }
                        }

                    LocalClipboardRead.Unavailable ->
                        clipboardWorkers.execute {
                            runCatching {
                                active.failClipboardRead(
                                    requestId,
                                    MobileClipboardPlatformFailure.UNAVAILABLE,
                                )
                            }
                        }

                    LocalClipboardRead.Failed ->
                        clipboardWorkers.execute {
                            runCatching {
                                active.failClipboardRead(
                                    requestId,
                                    MobileClipboardPlatformFailure.FAILED,
                                )
                            }
                        }
                }
            }

            MobileClipboardRequestKind.WRITE -> {
                val text = runCatching { request.takeText() }.getOrNull()
                if (text == null) {
                    clipboardWorkers.execute {
                        runCatching {
                            active.failClipboardWrite(
                                requestId,
                                MobileClipboardPlatformFailure.FAILED,
                            )
                        }
                    }
                    return
                }
                val write = clipboard.writeRemoteText(text)
                clipboardWorkers.execute {
                    when (write) {
                        LocalClipboardWrite.SUCCESS ->
                            runCatching { active.completeClipboardWrite(requestId) }

                        LocalClipboardWrite.FAILED ->
                            runCatching {
                                active.failClipboardWrite(
                                    requestId,
                                    MobileClipboardPlatformFailure.FAILED,
                                )
                            }
                    }
                }
            }
        }
    }

    private fun isCurrentAgent(
        active: MobileTrustedPresenceAgent,
        token: Long,
    ): Boolean =
        synchronized(lock) {
            !closed.get() && generation == token && agent === active
        }

    private fun publishAgentSnapshotLocked() {
        val active = agent ?: return
        runCatching {
            active.snapshot() to active.permissionSnapshot()
        }.onSuccess { (snapshot, permissions) ->
            publishLocked(snapshot.toRuntimeSnapshot(permissions))
        }
    }

    private fun failClosedPolicyLocked(active: MobileTrustedPresenceAgent) {
        if (runCatching { active.failClosedPolicy() }.isFailure) {
            stopAgentLocked()
            publishLocked(
                RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
            )
            return
        }
        publishAgentSnapshotLocked()
    }

    private fun publishLocked(snapshot: RuntimeSnapshot) {
        current = snapshot
        listeners.forEach { it(snapshot) }
    }
}

private fun MobilePresenceSnapshot.toRuntimeSnapshot(
    permissions: MobilePermissionSnapshot? = null,
): RuntimeSnapshot {
    val presence =
        when (phase) {
            MobilePresencePhase.DISCOVERING -> RuntimePresence.DISCOVERING
            MobilePresencePhase.CONNECTING -> RuntimePresence.CONNECTING
            MobilePresencePhase.ONLINE -> RuntimePresence.ONLINE
            MobilePresencePhase.RECONNECTING -> RuntimePresence.RECONNECTING
            MobilePresencePhase.PAUSED -> RuntimePresence.PAUSED
            MobilePresencePhase.FAILED -> RuntimePresence.FAILED
        }
    return runtime?.toRuntimeSnapshot(presence, permissions)
        ?: RuntimeSnapshot.disconnected().copy(
            presence = presence,
            policyRevision = permissions?.policyRevision ?: 0uL,
            permissionRules =
                permissions?.rules.orEmpty().map { rule ->
                    RuntimePermissionRule(
                        sourceDeviceId = rule.sourceDeviceId,
                        capabilityId = rule.capabilityId,
                        operation = rule.operation,
                        effect =
                            when (rule.effect) {
                                uniffi.crosslab_mobile_ffi.MobilePermissionEffect.ALLOW ->
                                    RuntimePermissionEffect.ALLOW
                                uniffi.crosslab_mobile_ffi.MobilePermissionEffect.DENY ->
                                    RuntimePermissionEffect.DENY
                                uniffi.crosslab_mobile_ffi.MobilePermissionEffect.ASK ->
                                    RuntimePermissionEffect.ASK
                            },
                    )
                },
        )
}

private fun MobileClipboardOutcome.toClipboardResult(): ClipboardResult =
    when (this) {
        MobileClipboardOutcome.SUCCESS -> ClipboardResult.SUCCESS
        MobileClipboardOutcome.NOT_CONNECTED -> ClipboardResult.NOT_CONNECTED
        MobileClipboardOutcome.NOT_NEGOTIATED -> ClipboardResult.NOT_NEGOTIATED
        MobileClipboardOutcome.OVERSIZED -> ClipboardResult.OVERSIZED
        MobileClipboardOutcome.RESOURCE_LIMIT -> ClipboardResult.RESOURCE_LIMIT
        MobileClipboardOutcome.TIMED_OUT -> ClipboardResult.TIMED_OUT
        MobileClipboardOutcome.CANCELLED -> ClipboardResult.CANCELLED
        MobileClipboardOutcome.DENIED -> ClipboardResult.DENIED
        MobileClipboardOutcome.UNAVAILABLE -> ClipboardResult.UNAVAILABLE
        MobileClipboardOutcome.FAILED -> ClipboardResult.FAILED
    }

private fun RuntimePermissionEffect.toMobilePermissionEffect(): MobilePermissionEffect =
    when (this) {
        RuntimePermissionEffect.ALLOW -> MobilePermissionEffect.ALLOW
        RuntimePermissionEffect.DENY -> MobilePermissionEffect.DENY
        RuntimePermissionEffect.ASK -> MobilePermissionEffect.ASK
    }
