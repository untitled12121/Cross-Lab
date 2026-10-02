package dev.crosslab.android.features.audit

import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong
import dev.crosslab.android.features.devices.RuntimeSnapshot
import dev.crosslab.android.features.filetransfer.FileTransferState
import dev.crosslab.android.features.pairing.PairingJoinerStage
import uniffi.crosslab_mobile_ffi.MobileAuditAction
import uniffi.crosslab_mobile_ffi.MobileAuditHistory
import uniffi.crosslab_mobile_ffi.MobileAuditOutcome

data class AuditRecordingStatus(
    val history: MobileAuditHistory?,
    val droppedEvents: Long,
    val unavailable: Boolean,
)

/** One bounded writer per application, independent of identity, policy and UI threads. */
class AndroidAuditRecorder(
    private val store: AndroidAuditStore,
) : AutoCloseable {
    private val tracker = AuditLifecycleTracker()
    private val dropped = AtomicLong()
    private val observers = CopyOnWriteArraySet<(AuditRecordingStatus) -> Unit>()
    private val writer = ThreadPoolExecutor(
        1, 1, 0L, TimeUnit.MILLISECONDS, ArrayBlockingQueue(64),
        { task -> Thread(task, "crosslab-owner-audit").apply { isDaemon = true } },
        ThreadPoolExecutor.AbortPolicy(),
    )
    @Volatile
    private var status = AuditRecordingStatus(null, 0L, false)

    init {
        writer.execute {
            val initial = runCatching { store.read() }
            publish(AuditRecordingStatus(initial.getOrNull(), dropped.get(), initial.isFailure))
        }
    }

    fun observe(listener: (AuditRecordingStatus) -> Unit): AutoCloseable {
        observers += listener
        listener(status)
        return AutoCloseable { observers -= listener }
    }

    internal fun pairing(stage: PairingJoinerStage) =
        enqueue(tracker.pairing(stage))

    internal fun runtime(snapshot: RuntimeSnapshot) =
        enqueue(tracker.runtime(snapshot))

    internal fun transfer(state: FileTransferState) =
        enqueue(tracker.transfer(state))

    internal fun subscription(active: Boolean) = enqueue(tracker.subscription(active))

    internal fun permissionCommitted(revision: ULong) =
        enqueue(tracker.permissionCommitted(revision))

    private fun enqueue(intents: List<AuditIntent>) {
        intents.forEach { record(it) }
    }

    fun record(
        action: MobileAuditAction,
        outcome: MobileAuditOutcome,
        revision: ULong = 0uL,
        callback: (Result<MobileAuditHistory>) -> Unit = {},
    ) = record(AuditIntent(action, outcome, revision), callback)

    private fun record(
        intent: AuditIntent,
        callback: (Result<MobileAuditHistory>) -> Unit = {},
    ) {
        try {
            writer.execute {
                val result = runCatching {
                    store.record(intent.action, intent.outcome, intent.revision)
                }
                if (result.isFailure) dropped.incrementAndGet()
                publish(AuditRecordingStatus(result.getOrNull(), dropped.get(), result.isFailure))
                runCatching { callback(result) }
            }
        } catch (_: RejectedExecutionException) {
            publish(AuditRecordingStatus(null, dropped.incrementAndGet(), true))
            callback(Result.failure(IllegalStateException("audit writer queue full")))
        }
    }

    /** Explicit export follows prior history writes and clears in recorder order. */
    fun export(callback: (Result<String>) -> Unit) {
        try {
            writer.execute {
                val result = runCatching { store.export() }
                if (result.isFailure) {
                    publish(AuditRecordingStatus(null, dropped.get(), true))
                }
                runCatching { callback(result) }
            }
        } catch (_: RejectedExecutionException) {
            publish(AuditRecordingStatus(null, dropped.incrementAndGet(), true))
            runCatching {
                callback(Result.failure(IllegalStateException("audit writer queue full")))
            }
        }
    }

    /** Explicit clear is ordered after already queued events, never races old writes. */
    fun clear(callback: (Result<MobileAuditHistory>) -> Unit) {
        try {
            writer.execute {
                val result = runCatching { store.clear() }
                publish(AuditRecordingStatus(result.getOrNull(), dropped.get(), result.isFailure))
                runCatching { callback(result) }
            }
        } catch (_: RejectedExecutionException) {
            publish(AuditRecordingStatus(null, dropped.incrementAndGet(), true))
            runCatching {
                callback(Result.failure(IllegalStateException("audit writer queue full")))
            }
        }
    }

    private fun publish(next: AuditRecordingStatus) {
        status = next
        observers.forEach { observer -> runCatching { observer(next) } }
    }

    override fun close() {
        writer.shutdown()
    }
}
