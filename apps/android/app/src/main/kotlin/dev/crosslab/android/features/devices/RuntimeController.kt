package dev.crosslab.android.features.devices

import java.util.concurrent.CopyOnWriteArraySet

interface RuntimePort {
    val peerControlAvailable: Boolean
        get() = false

    fun start()
    fun stop()
    fun networkLost()
    fun networkAvailable()
    fun disconnectPeer()
    fun reconnectPeer()

    fun setPermission(
        capabilityId: String,
        operation: String,
        effect: RuntimePermissionEffect,
    ): Boolean = false

    fun setPermissionForPeer(
        peerDeviceId: String,
        capabilityId: String,
        operation: String,
        effect: RuntimePermissionEffect,
    ): Boolean = false

    /** An opaque handoff token for owner dialogs; never expose raw session IDs. */
    fun sessionBinding(capabilityId: String): NativeSessionBinding? = null

    fun snapshot(): RuntimeSnapshot = RuntimeSnapshot.disconnected()

    fun observeSnapshots(listener: (RuntimeSnapshot) -> Unit): AutoCloseable {
        listener(snapshot())
        return AutoCloseable {}
    }

    fun shutdown() = Unit
}

enum class RuntimeLifecycle {
    STOPPED,
    RUNNING,
    SHUTDOWN,
}

data class RuntimeControllerState(
    val lifecycle: RuntimeLifecycle,
    val networkAvailable: Boolean,
    val peerControlAvailable: Boolean,
    val snapshot: RuntimeSnapshot,
) {
    companion object {
        fun initial(peerControlAvailable: Boolean = false): RuntimeControllerState =
            RuntimeControllerState(
                lifecycle = RuntimeLifecycle.STOPPED,
                networkAvailable = true,
                peerControlAvailable = peerControlAvailable,
                snapshot = RuntimeSnapshot.disconnected(),
            )
    }
}

class RuntimeController(
    private val port: RuntimePort,
) {
    private val lock = Any()
    private val listeners = CopyOnWriteArraySet<(RuntimeControllerState) -> Unit>()

    @Volatile
    private var current = RuntimeControllerState.initial(port.peerControlAvailable)

    private val portSubscription =
        port.observeSnapshots { snapshot ->
            update {
                if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
                it.copy(snapshot = snapshot)
            }
        }

    fun state(): RuntimeControllerState = current

    fun sessionBinding(capabilityId: String): NativeSessionBinding? {
        // The port may publish controller callbacks while holding its own lock.
        // Never hold the controller lock when requesting a port session ticket.
        val before = current
        if (before.lifecycle != RuntimeLifecycle.RUNNING || !before.networkAvailable ||
            !before.snapshot.hasAuthenticatedCapability(capabilityId)
        ) return null
        val binding = port.sessionBinding(capabilityId) ?: return null
        val after = current
        if (after.lifecycle != RuntimeLifecycle.RUNNING || !after.networkAvailable ||
            !after.snapshot.hasAuthenticatedCapability(capabilityId) ||
            after.snapshot.peerDeviceId != before.snapshot.peerDeviceId
        ) return null
        return binding
    }

    fun observe(listener: (RuntimeControllerState) -> Unit): AutoCloseable {
        listeners += listener
        listener(current)
        return AutoCloseable { listeners -= listener }
    }

    fun onForeground() {
        update {
            if (it.lifecycle != RuntimeLifecycle.STOPPED) return@update null
            startPort(it)
        }
    }

    fun onBackground() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            if (runCatching { port.stop() }.isFailure) {
                // A failed stop cannot be treated as a safely paused runtime.
                runCatching { port.shutdown() }
                it.copy(
                    lifecycle = RuntimeLifecycle.SHUTDOWN,
                    snapshot = RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
                )
            } else {
                it.copy(
                    lifecycle = RuntimeLifecycle.STOPPED,
                    snapshot = RuntimeSnapshot.disconnected(),
                )
            }
        }
        if (current.lifecycle == RuntimeLifecycle.SHUTDOWN) {
            portSubscription.close()
        }
    }

    fun onNetworkLost() {
        update {
            if (it.lifecycle == RuntimeLifecycle.SHUTDOWN || !it.networkAvailable) {
                return@update null
            }
            if (it.lifecycle != RuntimeLifecycle.RUNNING) {
                return@update it.copy(networkAvailable = false)
            }
            val next = runCatching {
                port.networkLost()
                port.snapshot()
            }
            next.fold(
                onSuccess = { updated -> it.copy(networkAvailable = false, snapshot = updated) },
                onFailure = { _ -> failClosed(it, networkAvailable = false) },
            )
        }
    }

    fun onNetworkAvailable() {
        update {
            if (it.lifecycle == RuntimeLifecycle.SHUTDOWN || it.networkAvailable) {
                return@update null
            }
            if (it.lifecycle != RuntimeLifecycle.RUNNING) {
                return@update it.copy(networkAvailable = true)
            }
            val snapshot = runCatching {
                port.networkAvailable()
                port.snapshot()
            }
            snapshot.fold(
                onSuccess = { updated -> it.copy(networkAvailable = true, snapshot = updated) },
                onFailure = { _ -> failClosed(it, networkAvailable = true) },
            )
        }
    }

    fun disconnectPeer() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            runCatching {
                port.disconnectPeer()
                port.snapshot()
            }.fold(
                onSuccess = { snapshot -> it.copy(snapshot = snapshot) },
                onFailure = { _ -> failClosed(it) },
            )
        }
    }

    fun reconnectPeer() {
        update {
            when (it.lifecycle) {
                RuntimeLifecycle.STOPPED -> startPort(it)
                RuntimeLifecycle.RUNNING -> {
                    runCatching {
                        port.reconnectPeer()
                        port.snapshot()
                    }.fold(
                        onSuccess = { snapshot -> it.copy(snapshot = snapshot) },
                        onFailure = { _ -> failClosed(it) },
                    )
                }
                RuntimeLifecycle.SHUTDOWN -> null
            }
        }
    }

    fun setPermission(
        capabilityId: String,
        operation: String,
        effect: RuntimePermissionEffect,
    ): Boolean {
        var changed = false
        update {
            val snapshot = it.snapshot
            if (it.lifecycle != RuntimeLifecycle.RUNNING ||
                !it.networkAvailable ||
                snapshot.peerDeviceId == null ||
                snapshot.session != RuntimeSession.ACTIVE ||
                snapshot.trust != RuntimeTrust.TRUSTED ||
                snapshot.connectivity != RuntimeConnectivity.CONNECTED ||
                snapshot.security != RuntimeSecurity.AUTHENTICATED ||
                capabilityId !in snapshot.capabilityIds
            ) return@update null
            changed = port.setPermission(capabilityId, operation, effect)
            it.copy(snapshot = port.snapshot())
        }
        return changed
    }

    fun setPermissionForPeer(
        peerDeviceId: String,
        capabilityId: String,
        operation: String,
        effect: RuntimePermissionEffect,
    ): Boolean {
        var changed = false
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING ||
                !it.networkAvailable ||
                it.snapshot.session != RuntimeSession.ACTIVE ||
                it.snapshot.trust != RuntimeTrust.TRUSTED ||
                it.snapshot.connectivity != RuntimeConnectivity.CONNECTED ||
                it.snapshot.security != RuntimeSecurity.AUTHENTICATED ||
                capabilityId !in it.snapshot.capabilityIds ||
                it.snapshot.peerDeviceId != peerDeviceId
            ) return@update null
            changed = port.setPermissionForPeer(peerDeviceId, capabilityId, operation, effect)
            it.copy(snapshot = port.snapshot())
        }
        return changed
    }

    fun reloadTrust(): Boolean {
        var succeeded = true
        update {
            if (it.lifecycle == RuntimeLifecycle.SHUTDOWN) return@update null
            if (runCatching { port.stop() }.isFailure) {
                succeeded = false
                return@update failClosed(it)
            }
            if (it.lifecycle != RuntimeLifecycle.RUNNING) {
                return@update it.copy(snapshot = RuntimeSnapshot.disconnected())
            }
            val restarted = startPort(it)
            succeeded = restarted.lifecycle == RuntimeLifecycle.RUNNING
            restarted
        }
        return succeeded
    }

    fun shutdown() {
        var didShutdown = false
        update {
            if (it.lifecycle == RuntimeLifecycle.SHUTDOWN) return@update null
            if (it.lifecycle == RuntimeLifecycle.RUNNING) {
                runCatching { port.stop() }
            }
            didShutdown = true
            it.copy(
                lifecycle = RuntimeLifecycle.SHUTDOWN,
                snapshot = RuntimeSnapshot.disconnected(),
            )
        }
        if (didShutdown) {
            portSubscription.close()
            runCatching { port.shutdown() }
        }
    }

    private fun startPort(state: RuntimeControllerState): RuntimeControllerState {
        val result =
            runCatching {
                port.start()
                if (!state.networkAvailable) port.networkLost()
                port.snapshot()
            }
        return result.fold(
            onSuccess = { state.copy(lifecycle = RuntimeLifecycle.RUNNING, snapshot = it) },
            onFailure = { failClosed(state) },
        )
    }

    private fun failClosed(
        state: RuntimeControllerState,
        networkAvailable: Boolean = state.networkAvailable,
    ): RuntimeControllerState {
        val stopped = runCatching { port.stop() }.isSuccess
        if (!stopped) {
            runCatching { port.shutdown() }
            portSubscription.close()
        }
        return state.copy(
            lifecycle = if (stopped) RuntimeLifecycle.STOPPED else RuntimeLifecycle.SHUTDOWN,
            networkAvailable = networkAvailable,
            snapshot = RuntimeSnapshot.disconnected().copy(presence = RuntimePresence.FAILED),
        )
    }

    private fun update(transform: (RuntimeControllerState) -> RuntimeControllerState?) {
        val next =
            synchronized(lock) {
                val updated = transform(current) ?: return
                current = updated
                updated
            }
        listeners.forEach { it(next) }
    }
}
