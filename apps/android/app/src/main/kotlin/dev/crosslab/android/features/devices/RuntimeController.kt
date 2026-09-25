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
                if (it.lifecycle == RuntimeLifecycle.SHUTDOWN) return@update null
                it.copy(snapshot = snapshot)
            }
        }

    fun state(): RuntimeControllerState = current

    fun observe(listener: (RuntimeControllerState) -> Unit): AutoCloseable {
        listeners += listener
        listener(current)
        return AutoCloseable { listeners -= listener }
    }

    fun onForeground() {
        update {
            if (it.lifecycle != RuntimeLifecycle.STOPPED) return@update null
            port.start()
            it.copy(
                lifecycle = RuntimeLifecycle.RUNNING,
                snapshot = port.snapshot(),
            )
        }
    }

    fun onBackground() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.stop()
            it.copy(
                lifecycle = RuntimeLifecycle.STOPPED,
                snapshot = port.snapshot(),
            )
        }
    }

    fun onNetworkLost() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.networkLost()
            it.copy(
                networkAvailable = false,
                snapshot = port.snapshot(),
            )
        }
    }

    fun onNetworkAvailable() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.networkAvailable()
            it.copy(
                networkAvailable = true,
                snapshot = port.snapshot(),
            )
        }
    }

    fun disconnectPeer() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.disconnectPeer()
            it.copy(snapshot = port.snapshot())
        }
    }

    fun reconnectPeer() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.reconnectPeer()
            it.copy(snapshot = port.snapshot())
        }
    }

    fun setPermission(
        capabilityId: String,
        operation: String,
        effect: RuntimePermissionEffect,
    ): Boolean {
        var changed = false
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            changed = port.setPermission(capabilityId, operation, effect)
            it.copy(snapshot = port.snapshot())
        }
        return changed
    }

    fun shutdown() {
        var didShutdown = false
        update {
            if (it.lifecycle == RuntimeLifecycle.SHUTDOWN) return@update null
            if (it.lifecycle == RuntimeLifecycle.RUNNING) {
                port.stop()
            }
            didShutdown = true
            it.copy(
                lifecycle = RuntimeLifecycle.SHUTDOWN,
                snapshot = port.snapshot(),
            )
        }
        if (didShutdown) {
            portSubscription.close()
            port.shutdown()
        }
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
