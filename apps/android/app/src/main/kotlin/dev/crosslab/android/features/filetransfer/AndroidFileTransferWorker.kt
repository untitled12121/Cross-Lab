package dev.crosslab.android.features.filetransfer

import android.net.Uri
import java.io.InputStream
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobileFileTransferCancellationKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferDataEvent
import uniffi.crosslab_mobile_ffi.MobileFileTransferDataKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferRequest
import uniffi.crosslab_mobile_ffi.MobileFileTransferRetainedKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferState
import uniffi.crosslab_mobile_ffi.MobileFileTransferTerminalOutcome
import uniffi.crosslab_mobile_ffi.MobileTrustedPresenceAgent

private const val FILE_TRANSFER_WAIT_MS = 60_000uL
private const val FILE_TRANSFER_IO_BYTES = 64 * 1024
private const val FILE_TRANSFER_CHECKPOINT_BYTES = 1_048_576L

sealed interface AndroidIncomingFileTransfer {
    val transferId: ByteArray
    val displayName: String
    val fileSize: ULong

    data class NeedsDestination(
        val requestId: ByteArray,
        override val transferId: ByteArray,
        override val displayName: String,
        override val fileSize: ULong,
    ) : AndroidIncomingFileTransfer

    data class Resuming(
        override val transferId: ByteArray,
        override val displayName: String,
        override val fileSize: ULong,
        val resumeOffset: ULong,
    ) : AndroidIncomingFileTransfer
}

sealed interface AndroidFileTransferReceiveStatus {
    val transferId: ByteArray

    data class Ready(
        override val transferId: ByteArray,
        val resumeOffset: ULong,
        val totalBytes: ULong,
    ) : AndroidFileTransferReceiveStatus

    data class Receiving(
        override val transferId: ByteArray,
        val receivedBytes: ULong,
        val totalBytes: ULong,
    ) : AndroidFileTransferReceiveStatus

    data class Completed(
        override val transferId: ByteArray,
        val totalBytes: ULong,
    ) : AndroidFileTransferReceiveStatus

    data class AlreadyComplete(
        override val transferId: ByteArray,
        val totalBytes: ULong,
    ) : AndroidFileTransferReceiveStatus

    data class Cancelled(
        override val transferId: ByteArray,
        val receivedBytes: ULong,
        val totalBytes: ULong,
    ) : AndroidFileTransferReceiveStatus

    data class Failed(
        override val transferId: ByteArray,
        val receivedBytes: ULong,
        val totalBytes: ULong,
        val failure: AndroidFileTransferFailure,
    ) : AndroidFileTransferReceiveStatus
}

enum class AndroidFileTransferFailure {
    CONNECTION,
    STORAGE,
    INTEGRITY,
    INVALID_STATE,
}

class AndroidFileTransferWorker(
    private val agent: MobileTrustedPresenceAgent,
    private val adapter: AndroidFileTransferAdapter,
    private val stateStore: AndroidFileTransferStateStore,
) : AutoCloseable {
    private val closed = AtomicBoolean(false)
    private val lock = Any()
    private val listeners = CopyOnWriteArraySet<(AndroidFileTransferReceiveStatus) -> Unit>()
    private val pendingListeners = CopyOnWriteArraySet<(AndroidIncomingFileTransfer.NeedsDestination) -> Unit>()
    private val requestEvents = singleThread("crosslab-file-request")
    private val cancellationEvents = singleThread("crosslab-file-cancel")
    private val dataEvents = singleThread("crosslab-file-data")
    private val storage = singleThread("crosslab-file-storage")
    private val retained =
        MobileFileTransferState(
            encoded = stateStore.load(),
        )

    private val pending = LinkedHashMap<String, MobileFileTransferRequest>()
    private val active = LinkedHashMap<String, ActiveReceive>()

    fun start() {
        if (closed.get()) return
        requestEvents.execute(::requestLoop)
        cancellationEvents.execute(::cancellationLoop)
        dataEvents.execute(::dataLoop)
    }

    fun observeStatus(listener: (AndroidFileTransferReceiveStatus) -> Unit): AutoCloseable {
        listeners += listener
        return AutoCloseable { listeners -= listener }
    }

    fun observePending(listener: (AndroidIncomingFileTransfer.NeedsDestination) -> Unit): AutoCloseable {
        pendingListeners += listener
        return AutoCloseable { pendingListeners -= listener }
    }

    fun acceptDestination(
        requestId: ByteArray,
        destination: Uri,
    ): Boolean {
        val request =
            synchronized(lock) {
                pending.remove(requestId.key())
            } ?: return false

        storage.execute {
            acceptNewDestination(request, destination)
        }
        return true
    }

    fun decline(requestId: ByteArray): Boolean {
        val request =
            synchronized(lock) {
                pending.remove(requestId.key())
            } ?: return false

        storage.execute {
            runCatching { agent.declineFileTransferRequest(request.requestId()) }
            publish(
                AndroidFileTransferReceiveStatus.Cancelled(
                    transferId = request.transferId(),
                    receivedBytes = 0uL,
                    totalBytes = request.fileSize(),
                ),
            )
        }
        return true
    }

    fun cancelReceive(transferId: ByteArray): Boolean {
        val activeReceive =
            synchronized(lock) {
                active.remove(transferId.key())
            } ?: return false

        storage.execute {
            runCatching { agent.cancelFileTransferReceive(transferId) }
            activeReceive.close()
            publish(
                AndroidFileTransferReceiveStatus.Cancelled(
                    transferId = transferId,
                    receivedBytes = activeReceive.offset.toULong(),
                    totalBytes = activeReceive.request.fileSize(),
                ),
            )
        }
        return true
    }

    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        requestEvents.shutdownNow()
        cancellationEvents.shutdownNow()
        dataEvents.shutdownNow()
        storage.shutdownNow()
        synchronized(lock) {
            pending.clear()
            active.values.forEach(ActiveReceive::close)
            active.clear()
        }
        listeners.clear()
        pendingListeners.clear()
    }

    private fun requestLoop() {
        while (!closed.get()) {
            val request =
                try {
                    agent.waitFileTransferRequest(FILE_TRANSFER_WAIT_MS)
                } catch (_: Exception) {
                    return
                } ?: continue

            storage.execute {
                handleRequest(request)
            }
        }
    }

    private fun cancellationLoop() {
        while (!closed.get()) {
            val cancellation =
                try {
                    agent.waitFileTransferCancellation(FILE_TRANSFER_WAIT_MS)
                } catch (_: Exception) {
                    return
                } ?: continue

            when (cancellation.kind) {
                MobileFileTransferCancellationKind.REQUEST -> {
                    val requestId = cancellation.requestId ?: continue
                    synchronized(lock) {
                        pending.remove(requestId.key())
                    }
                }

                MobileFileTransferCancellationKind.TRANSFER -> {
                    val receive =
                        synchronized(lock) {
                            active.remove(cancellation.transferId.key())
                        }
                    if (receive != null) {
                        storage.execute {
                            receive.close()
                            publish(
                                AndroidFileTransferReceiveStatus.Failed(
                                    transferId = cancellation.transferId,
                                    receivedBytes = receive.offset.toULong(),
                                    totalBytes = receive.request.fileSize(),
                                    failure = AndroidFileTransferFailure.CONNECTION,
                                ),
                            )
                        }
                    }
                }
            }
        }
    }

    private fun dataLoop() {
        while (!closed.get()) {
            val event =
                try {
                    agent.waitFileTransferData(FILE_TRANSFER_WAIT_MS)
                } catch (_: Exception) {
                    return
                } ?: continue
            storage.execute {
                handleData(event)
            }
        }
    }

    private fun handleRequest(request: MobileFileTransferRequest) {
        if (closed.get()) return
        val transferId = request.transferId()
        val partialLength = adapter.partialLength(transferId)?.toULong()
        val matched =
            runCatching {
                retained.matchRequest(
                    request = request,
                    partialFileLen = partialLength,
                )
            }.getOrElse {
                failRequest(request, AndroidFileTransferFailure.INVALID_STATE)
                return
            }

        when (matched.kind) {
            MobileFileTransferRetainedKind.NEW -> {
                val pendingTransfer =
                    AndroidIncomingFileTransfer.NeedsDestination(
                        requestId = request.requestId(),
                        transferId = transferId,
                        displayName = request.displayName(),
                        fileSize = request.fileSize(),
                    )
                synchronized(lock) {
                    pending[request.requestId().key()] = request
                }
                pendingListeners.forEach { it(pendingTransfer) }
            }

            MobileFileTransferRetainedKind.ALREADY_COMPLETE -> {
                val completed =
                    runCatching {
                        agent.completeFileTransferAlreadyComplete(request.requestId())
                    }.isSuccess
                if (completed) {
                    publish(
                        AndroidFileTransferReceiveStatus.AlreadyComplete(
                            transferId = transferId,
                            totalBytes = request.fileSize(),
                        ),
                    )
                } else {
                    failRequest(request, AndroidFileTransferFailure.CONNECTION)
                }
            }

            MobileFileTransferRetainedKind.PARTIAL -> {
                recoverPartial(request, matched.durableOffset, matched.localLocator)
            }
        }
    }

    private fun acceptNewDestination(
        request: MobileFileTransferRequest,
        destination: Uri,
    ) {
        val locator = adapter.encodeLocalLocator(destination)
        if (locator == null) {
            failRequest(request, AndroidFileTransferFailure.STORAGE)
            return
        }

        adapter.persistDestinationGrant(destination)
        val transferId = request.transferId()
        val partial = adapter.openPartial(transferId, 0)
        if (partial == null) {
            failRequest(request, AndroidFileTransferFailure.STORAGE)
            return
        }

        val now = unixNow()
        val stored =
            runCatching {
                retained.upsertPartial(
                    request = request,
                    durableOffset = 0uL,
                    localLocator = locator,
                    updatedAtUnixSecs = now,
                )
                stateStore.commit(retained.encode())
            }.isSuccess
        if (!stored) {
            partial.close()
            failRequest(request, AndroidFileTransferFailure.STORAGE)
            return
        }

        val receive =
            ActiveReceive(
                request = request,
                destination = destination,
                localLocator = locator,
                partial = partial,
                offset = 0,
                durableOffset = 0,
            )
        synchronized(lock) {
            active[transferId.key()] = receive
        }
        if (!sendReady(receive)) {
            releaseActive(transferId)
        }
    }

    private fun recoverPartial(
        request: MobileFileTransferRequest,
        durableOffset: ULong?,
        localLocator: ByteArray?,
    ) {
        val durable = durableOffset?.toLongExact() ?: run {
            failRequest(request, AndroidFileTransferFailure.INVALID_STATE)
            return
        }
        val locator = localLocator ?: run {
            failRequest(request, AndroidFileTransferFailure.INVALID_STATE)
            return
        }
        val destination = adapter.decodeLocalLocator(locator) ?: run {
            failRequest(request, AndroidFileTransferFailure.INVALID_STATE)
            return
        }

        if (
            durableOffset == request.fileSize() &&
            verifyUri(destination, request)
        ) {
            val completed =
                runCatching {
                    retained.markCompleted(request, unixNow())
                    stateStore.commit(retained.encode())
                    adapter.discardPartial(request.transferId())
                    agent.completeFileTransferAlreadyComplete(request.requestId())
                }.isSuccess
            if (completed) {
                publish(
                    AndroidFileTransferReceiveStatus.AlreadyComplete(
                        transferId = request.transferId(),
                        totalBytes = request.fileSize(),
                    ),
                )
                return
            }
        }

        val partial = adapter.openPartial(request.transferId(), durable) ?: run {
            failRequest(request, AndroidFileTransferFailure.STORAGE)
            return
        }
        val receive =
            ActiveReceive(
                request = request,
                destination = destination,
                localLocator = locator,
                partial = partial,
                offset = durable,
                durableOffset = durable,
            )
        synchronized(lock) {
            active[request.transferId().key()] = receive
        }
        if (!sendReady(receive)) {
            releaseActive(request.transferId())
        }
    }

    private fun sendReady(receive: ActiveReceive): Boolean {
        val ready =
            runCatching {
                agent.completeFileTransferReady(
                    receive.request.requestId(),
                    receive.offset.toULong(),
                )
            }.isSuccess
        if (!ready) {
            publish(
                AndroidFileTransferReceiveStatus.Failed(
                    transferId = receive.request.transferId(),
                    receivedBytes = receive.offset.toULong(),
                    totalBytes = receive.request.fileSize(),
                    failure = AndroidFileTransferFailure.CONNECTION,
                ),
            )
            return false
        }

        publish(
            AndroidFileTransferReceiveStatus.Ready(
                transferId = receive.request.transferId(),
                resumeOffset = receive.offset.toULong(),
                totalBytes = receive.request.fileSize(),
            ),
        )
        return true
    }

    private fun handleData(event: MobileFileTransferDataEvent) {
        val transferId = event.transferId()
        val receive =
            synchronized(lock) {
                active[transferId.key()]
            } ?: return

        when (event.kind()) {
            MobileFileTransferDataKind.OPENED -> handleOpened(receive, event)
            MobileFileTransferDataKind.CHUNK -> handleChunk(receive, event)
            MobileFileTransferDataKind.FINISHED -> handleFinished(receive, event)
            MobileFileTransferDataKind.CANCELLED -> {
                releaseActive(transferId)
                publish(
                    AndroidFileTransferReceiveStatus.Cancelled(
                        transferId = transferId,
                        receivedBytes = receive.offset.toULong(),
                        totalBytes = receive.request.fileSize(),
                    ),
                )
            }
        }
    }

    private fun handleOpened(
        receive: ActiveReceive,
        event: MobileFileTransferDataEvent,
    ) {
        val resumeOffset = event.resumeOffset()?.toLongExact()
        if (resumeOffset == null || resumeOffset != receive.offset) {
            abort(receive, event.streamId(), MobileFileTransferTerminalOutcome.STORAGE_FAILED)
            return
        }
        receive.streamId = event.streamId()
    }

    private fun handleChunk(
        receive: ActiveReceive,
        event: MobileFileTransferDataEvent,
    ) {
        val streamId = receive.streamId
        if (streamId == null || !streamId.contentEquals(event.streamId())) {
            return
        }
        val bytes =
            runCatching { event.takeBytes() }
                .getOrNull()
                ?: run {
                    abort(receive, streamId, MobileFileTransferTerminalOutcome.STORAGE_FAILED)
                    return
                }

        val next = receive.offset + bytes.size
        val total = receive.request.fileSize().toLongExact()
        if (total == null || next < receive.offset || next > total || !receive.partial.append(bytes)) {
            abort(receive, streamId, MobileFileTransferTerminalOutcome.STORAGE_FAILED)
            return
        }
        receive.offset = next

        val durable = durableOffset(next, total)
        if (durable > receive.durableOffset) {
            if (!receive.partial.force()) {
                abort(receive, streamId, MobileFileTransferTerminalOutcome.STORAGE_FAILED)
                return
            }
            val persisted =
                runCatching {
                    retained.upsertPartial(
                        request = receive.request,
                        durableOffset = durable.toULong(),
                        localLocator = receive.localLocator,
                        updatedAtUnixSecs = unixNow(),
                    )
                    stateStore.commit(retained.encode())
                }.isSuccess
            if (!persisted) {
                abort(receive, streamId, MobileFileTransferTerminalOutcome.STORAGE_FAILED)
                return
            }
            receive.durableOffset = durable
        }

        publish(
            AndroidFileTransferReceiveStatus.Receiving(
                transferId = receive.request.transferId(),
                receivedBytes = receive.offset.toULong(),
                totalBytes = receive.request.fileSize(),
            ),
        )
    }

    private fun handleFinished(
        receive: ActiveReceive,
        event: MobileFileTransferDataEvent,
    ) {
        val streamId = receive.streamId
        if (streamId == null || !streamId.contentEquals(event.streamId())) return
        val total = receive.request.fileSize().toLongExact()
        if (total == null || receive.offset != total || !receive.partial.force()) {
            abort(receive, streamId, MobileFileTransferTerminalOutcome.STORAGE_FAILED)
            return
        }

        if (!verifyPartial(receive)) {
            releaseActive(receive.request.transferId())
            runCatching {
                retained.remove(receive.request.transferId())
                stateStore.commit(retained.encode())
                adapter.discardPartial(receive.request.transferId())
            }
            runCatching {
                agent.failFileTransferReceive(
                    streamId,
                    receive.request.transferId(),
                    MobileFileTransferTerminalOutcome.INTEGRITY_FAILED,
                )
            }
            publish(
                AndroidFileTransferReceiveStatus.Failed(
                    transferId = receive.request.transferId(),
                    receivedBytes = receive.offset.toULong(),
                    totalBytes = receive.request.fileSize(),
                    failure = AndroidFileTransferFailure.INTEGRITY,
                ),
            )
            return
        }

        if (!adapter.publishPartial(receive.request.transferId(), receive.destination)) {
            abort(receive, streamId, MobileFileTransferTerminalOutcome.STORAGE_FAILED)
            return
        }

        val committed =
            runCatching {
                retained.markCompleted(receive.request, unixNow())
                stateStore.commit(retained.encode())
            }.isSuccess
        if (!committed) {
            abort(receive, streamId, MobileFileTransferTerminalOutcome.STORAGE_FAILED)
            return
        }

        releaseActive(receive.request.transferId())
        adapter.discardPartial(receive.request.transferId())
        runCatching {
            agent.completeFileTransferResult(
                receive.request.transferId(),
                MobileFileTransferTerminalOutcome.COMPLETED,
            )
        }
        publish(
            AndroidFileTransferReceiveStatus.Completed(
                transferId = receive.request.transferId(),
                totalBytes = receive.request.fileSize(),
            ),
        )
    }

    private fun verifyPartial(receive: ActiveReceive): Boolean =
        adapter.openPartialForVerify(receive.request.transferId())?.use { input ->
            verifyStream(input, receive.request)
        } ?: false

    private fun verifyUri(
        uri: Uri,
        request: MobileFileTransferRequest,
    ): Boolean =
        adapter.openForRead(uri)?.use { input ->
            verifyStream(input, request)
        } ?: false

    private fun verifyStream(
        input: InputStream,
        request: MobileFileTransferRequest,
    ): Boolean {
        val verifier = request.verifier()
        val buffer = ByteArray(FILE_TRANSFER_IO_BYTES)
        while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            if (read == 0) continue
            if (
                runCatching {
                    verifier.update(buffer.copyOf(read))
                }.isFailure
            ) {
                return false
            }
        }
        return runCatching { verifier.finish() }.isSuccess
    }

    private fun abort(
        receive: ActiveReceive,
        streamId: ByteArray,
        outcome: MobileFileTransferTerminalOutcome,
    ) {
        releaseActive(receive.request.transferId())
        runCatching {
            agent.failFileTransferReceive(
                streamId,
                receive.request.transferId(),
                outcome,
            )
        }
        publish(
            AndroidFileTransferReceiveStatus.Failed(
                transferId = receive.request.transferId(),
                receivedBytes = receive.offset.toULong(),
                totalBytes = receive.request.fileSize(),
                failure =
                    when (outcome) {
                        MobileFileTransferTerminalOutcome.INTEGRITY_FAILED ->
                            AndroidFileTransferFailure.INTEGRITY

                        MobileFileTransferTerminalOutcome.STORAGE_FAILED ->
                            AndroidFileTransferFailure.STORAGE

                        else -> AndroidFileTransferFailure.CONNECTION
                    },
            ),
        )
    }

    private fun failRequest(
        request: MobileFileTransferRequest,
        failure: AndroidFileTransferFailure,
    ) {
        runCatching { agent.declineFileTransferRequest(request.requestId()) }
        publish(
            AndroidFileTransferReceiveStatus.Failed(
                transferId = request.transferId(),
                receivedBytes = 0uL,
                totalBytes = request.fileSize(),
                failure = failure,
            ),
        )
    }

    private fun releaseActive(transferId: ByteArray) {
        val receive =
            synchronized(lock) {
                active.remove(transferId.key())
            }
        receive?.close()
    }

    private fun publish(status: AndroidFileTransferReceiveStatus) {
        listeners.forEach { it(status) }
    }

    private data class ActiveReceive(
        val request: MobileFileTransferRequest,
        val destination: Uri,
        val localLocator: ByteArray,
        val partial: AndroidPartialFile,
        var offset: Long,
        var durableOffset: Long,
        var streamId: ByteArray? = null,
    ) {
        fun close() {
            runCatching { partial.close() }
        }
    }
}

internal fun durableOffset(
    offset: Long,
    fileSize: Long,
): Long =
    if (offset == fileSize) {
        offset
    } else {
        offset - (offset % FILE_TRANSFER_CHECKPOINT_BYTES)
    }

private fun unixNow(): ULong =
    (System.currentTimeMillis().coerceAtLeast(0L) / 1_000L).toULong()

private fun ULong.toLongExact(): Long? =
    takeIf { it <= Long.MAX_VALUE.toULong() }?.toLong()

private fun ByteArray.key(): String {
    val digits = "0123456789abcdef"
    return buildString(size * 2) {
        forEach { byte ->
            val value = byte.toInt() and 0xff
            append(digits[value ushr 4])
            append(digits[value and 0x0f])
        }
    }
}

private fun singleThread(name: String): ExecutorService =
    Executors.newSingleThreadExecutor { task ->
        Thread(task, name).apply { isDaemon = true }
    }
