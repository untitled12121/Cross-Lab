package dev.crosslab.android.features.devices

import java.util.concurrent.CopyOnWriteArraySet

interface RuntimePort {
    fun start()
    fun stop()
    fun networkLost()
    fun networkAvailable()
}

enum class RuntimeLifecycle {
    STOPPED,
    RUNNING,
    SHUTDOWN,
}

data class RuntimeControllerState(
    val lifecycle: RuntimeLifecycle,
    val networkAvailable: Boolean,
) {
    companion object {
        fun initial(): RuntimeControllerState =
            RuntimeControllerState(
                lifecycle = RuntimeLifecycle.STOPPED,
                networkAvailable = true,
            )
    }
}

class RuntimeController(
    private val port: RuntimePort,
) {
    private val lock = Any()
    private val listeners = CopyOnWriteArraySet<(RuntimeControllerState) -> Unit>()

    @Volatile
    private var current = RuntimeControllerState.initial()

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
            it.copy(lifecycle = RuntimeLifecycle.RUNNING)
        }
    }

    fun onBackground() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.stop()
            it.copy(lifecycle = RuntimeLifecycle.STOPPED)
        }
    }

    fun onNetworkLost() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.networkLost()
            it.copy(networkAvailable = false)
        }
    }

    fun onNetworkAvailable() {
        update {
            if (it.lifecycle != RuntimeLifecycle.RUNNING) return@update null
            port.networkAvailable()
            it.copy(networkAvailable = true)
        }
    }

    fun shutdown() {
        update {
            if (it.lifecycle == RuntimeLifecycle.SHUTDOWN) return@update null
            if (it.lifecycle == RuntimeLifecycle.RUNNING) {
                port.stop()
            }
            it.copy(lifecycle = RuntimeLifecycle.SHUTDOWN)
        }
    }

    private fun update(transform: (RuntimeControllerState) -> RuntimeControllerState?) {
        val next = synchronized(lock) {
            val updated = transform(current) ?: return
            current = updated
            updated
        }
        listeners.forEach { it(next) }
    }
}
