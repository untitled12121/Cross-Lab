package dev.crosslab.android.features.clipboard

import java.util.concurrent.CopyOnWriteArraySet

enum class ClipboardAction {
    SEND,
    FETCH,
}

enum class ClipboardResult {
    SUCCESS,
    UNAVAILABLE,
    NOT_CONNECTED,
    NOT_NEGOTIATED,
    OVERSIZED,
    RESOURCE_LIMIT,
    TIMED_OUT,
    CANCELLED,
    DENIED,
    FAILED,
}

data class ClipboardState(
    val available: Boolean,
    val busy: Boolean,
    val action: ClipboardAction?,
    val result: ClipboardResult?,
) {
    companion object {
        fun initial(available: Boolean): ClipboardState =
            ClipboardState(
                available = available,
                busy = false,
                action = null,
                result = null,
            )
    }
}

interface ClipboardPort {
    val clipboardAvailable: Boolean
        get() = false

    fun sendClipboard(onComplete: (ClipboardResult) -> Unit): Boolean = false

    fun fetchClipboard(onComplete: (ClipboardResult) -> Unit): Boolean = false
}

object UnavailableClipboardPort : ClipboardPort

class ClipboardController(
    private val port: ClipboardPort,
) {
    private val lock = Any()
    private val listeners = CopyOnWriteArraySet<(ClipboardState) -> Unit>()

    @Volatile
    private var current = ClipboardState.initial(port.clipboardAvailable)

    fun state(): ClipboardState = current

    fun observe(listener: (ClipboardState) -> Unit): AutoCloseable {
        listeners += listener
        listener(current)
        return AutoCloseable { listeners -= listener }
    }

    fun send() {
        start(ClipboardAction.SEND, port::sendClipboard)
    }

    fun fetch() {
        start(ClipboardAction.FETCH, port::fetchClipboard)
    }

    fun dismissResult() {
        update { state ->
            if (state.busy || state.result == null) return@update null
            state.copy(action = null, result = null)
        }
    }

    fun shutdown() {
        listeners.clear()
    }

    private fun start(
        action: ClipboardAction,
        operation: ((ClipboardResult) -> Unit) -> Boolean,
    ) {
        val started =
            update { state ->
                if (!state.available || state.busy) return@update null
                state.copy(busy = true, action = action, result = null)
            }
        if (!started) return

        if (!operation { result -> finish(action, result) }) {
            finish(action, ClipboardResult.UNAVAILABLE)
        }
    }

    private fun finish(
        action: ClipboardAction,
        result: ClipboardResult,
    ) {
        update { state ->
            if (!state.busy || state.action != action) return@update null
            state.copy(busy = false, result = result)
        }
    }

    private fun update(transform: (ClipboardState) -> ClipboardState?): Boolean {
        val next =
            synchronized(lock) {
                val updated = transform(current) ?: return false
                current = updated
                updated
            }
        listeners.forEach { it(next) }
        return true
    }
}
