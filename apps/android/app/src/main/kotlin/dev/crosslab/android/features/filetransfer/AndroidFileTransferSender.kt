package dev.crosslab.android.features.filetransfer

import android.net.Uri
import java.io.InputStream
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobileFileTransferAcceptanceKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferChunkOutcome
import uniffi.crosslab_mobile_ffi.MobileFileTransferHasher
import uniffi.crosslab_mobile_ffi.MobileFileTransferTerminalOutcome
import uniffi.crosslab_mobile_ffi.MobilePreparedFileTransfer
import uniffi.crosslab_mobile_ffi.MobileTrustedPresenceAgent
import uniffi.crosslab_mobile_ffi.fileTransferHasherForRetry

private const val FILE_TRANSFER_SEND_IO_BYTES = 64 * 1024
private const val BACKPRESSURE_RETRY_MS = 10L

sealed interface AndroidFileTransferSendStatus {
    val transferId: ByteArray?

    data object Idle : AndroidFileTransferSendStatus {
        override val transferId: ByteArray? = null
    }

    data class Preparing(
        override val transferId: ByteArray?,
        val hashedBytes: ULong,
    ) : AndroidFileTransferSendStatus

    data class WaitingForPeer(
        override val transferId: ByteArray,
        val totalBytes: ULong,
    ) : AndroidFileTransferSendStatus

    data class Sending(
        override val transferId: ByteArray,
        val sentBytes: ULong,
        val totalBytes: ULong,
    ) : AndroidFileTransferSendStatus

    data class Finalizing(
        override val transferId: ByteArray,
        val totalBytes: ULong,
    ) : AndroidFileTransferSendStatus

    data class Completed(
        override val transferId: ByteArray,
        val totalBytes: ULong,
    ) : AndroidFileTransferSendStatus

    data class Cancelled(
        override val transferId: ByteArray?,
        val sentBytes: ULong,
        val totalBytes: ULong?,
    ) : AndroidFileTransferSendStatus

    data class Failed(
        override val transferId: ByteArray?,
        val sentBytes: ULong,
        val totalBytes: ULong?,
        val failure: AndroidFileTransferSendFailure,
    ) : AndroidFileTransferSendStatus
}

enum class AndroidFileTransferSendFailure {
    SOURCE,
    SOURCE_CHANGED,
    CONNECTION,
    PEER_DENIED,
    INTEGRITY,
    STORAGE,
    INTERNAL,
}

class AndroidFileTransferSender(
    private val agent: MobileTrustedPresenceAgent,
    private val adapter: AndroidFileTransferAdapter,
) : AutoCloseable {
    private val executor: ExecutorService =
        Executors.newSingleThreadExecutor { task ->
            Thread(task, "crosslab-file-send").apply { isDaemon = true }
        }
    private val listeners = CopyOnWriteArraySet<(AndroidFileTransferSendStatus) -> Unit>()
    private val closed = AtomicBoolean(false)
    private val cancelled = AtomicBoolean(false)
    private val busy = AtomicBoolean(false)
    private val lock = Any()

    @Volatile
    private var retryToken: RetryToken? = null

    fun observe(listener: (AndroidFileTransferSendStatus) -> Unit): AutoCloseable {
        listeners += listener
        return AutoCloseable { listeners -= listener }
    }

    fun send(uri: Uri): Boolean {
        if (closed.get() || !busy.compareAndSet(false, true)) return false
        cancelled.set(false)
        executor.execute {
            try {
                sendFresh(uri)
            } finally {
                busy.set(false)
            }
        }
        return true
    }

    fun retry(): Boolean {
        val token = retryToken ?: return false
        if (closed.get() || !busy.compareAndSet(false, true)) return false
        cancelled.set(false)
        executor.execute {
            try {
                sendRetry(token)
            } finally {
                busy.set(false)
            }
        }
        return true
    }

    fun cancel() {
        cancelled.set(true)
    }

    fun discardRetry() {
        if (busy.get()) return
        synchronized(lock) {
            retryToken = null
        }
    }

    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        cancelled.set(true)
        executor.shutdownNow()
        synchronized(lock) {
            retryToken = null
        }
        listeners.clear()
    }

    private fun sendFresh(uri: Uri) {
        synchronized(lock) {
            retryToken = null
        }
        val source = adapter.inspectSource(uri)
        if (source == null) {
            fail(null, 0, null, AndroidFileTransferSendFailure.SOURCE)
            return
        }
        adapter.persistSourceGrant(uri)

        val prepared =
            prepare(
                source = source,
                hasher = MobileFileTransferHasher(),
                transferId = null,
            ) ?: return
        if (prepared.fileSize() != source.fileSize.toULong()) {
            fail(
                prepared.transferId(),
                0,
                prepared.fileSize(),
                AndroidFileTransferSendFailure.SOURCE_CHANGED,
            )
            return
        }
        val token =
            RetryToken(
                source = source,
                prepared = prepared,
            )
        synchronized(lock) {
            retryToken = token
        }
        sendPrepared(token)
    }

    private fun sendRetry(token: RetryToken) {
        val hasher =
            runCatching {
                fileTransferHasherForRetry(token.prepared.transferId())
            }.getOrElse {
                fail(
                    token.prepared.transferId(),
                    0,
                    token.prepared.fileSize(),
                    AndroidFileTransferSendFailure.INTERNAL,
                )
                return
            }
        val prepared =
            prepare(
                source = token.source,
                hasher = hasher,
                transferId = token.prepared.transferId(),
                displayName = token.prepared.displayName(),
            ) ?: return

        val same =
            runCatching {
                token.prepared.sameOffer(prepared)
            }.getOrDefault(false)
        if (!same) {
            synchronized(lock) {
                if (retryToken === token) retryToken = null
            }
            fail(
                token.prepared.transferId(),
                0,
                token.prepared.fileSize(),
                AndroidFileTransferSendFailure.SOURCE_CHANGED,
            )
            return
        }

        sendPrepared(
            token.copy(
                prepared = prepared,
            ),
        )
    }

    private fun prepare(
        source: AndroidFileTransferSource,
        hasher: MobileFileTransferHasher,
        transferId: ByteArray?,
        displayName: String = source.displayName,
    ): MobilePreparedFileTransfer? {
        val input = adapter.openSource(source)
        if (input == null) {
            fail(transferId, 0, source.fileSize.toULong(), AndroidFileTransferSendFailure.SOURCE)
            return null
        }

        input.use { stream ->
            val buffer = ByteArray(FILE_TRANSFER_SEND_IO_BYTES)
            var hashed = 0L
            while (true) {
                if (cancelled.get() || closed.get()) {
                    publish(
                        AndroidFileTransferSendStatus.Cancelled(
                            transferId = transferId,
                            sentBytes = 0uL,
                            totalBytes = source.fileSize.toULong(),
                        ),
                    )
                    return null
                }
                val read =
                    runCatching { stream.read(buffer) }
                        .getOrElse {
                            fail(
                                transferId,
                                0,
                                source.fileSize.toULong(),
                                AndroidFileTransferSendFailure.SOURCE,
                            )
                            return null
                        }
                if (read < 0) break
                if (read == 0) continue

                val bytes = buffer.copyOf(read)
                val updated =
                    runCatching {
                        hasher.update(bytes)
                    }.isSuccess
                if (!updated) {
                    fail(
                        transferId,
                        0,
                        source.fileSize.toULong(),
                        AndroidFileTransferSendFailure.INTERNAL,
                    )
                    return null
                }
                hashed += read
                publish(
                    AndroidFileTransferSendStatus.Preparing(
                        transferId = transferId,
                        hashedBytes = hashed.toULong(),
                    ),
                )
            }
        }

        return runCatching {
            hasher.finish(displayName)
        }.getOrElse {
            fail(
                transferId,
                0,
                source.fileSize.toULong(),
                AndroidFileTransferSendFailure.SOURCE,
            )
            null
        }
    }

    private fun sendPrepared(token: RetryToken) {
        val prepared = token.prepared
        val transferId = prepared.transferId()
        val totalBytes = prepared.fileSize()

        if (cancelled.get() || closed.get()) {
            publish(
                AndroidFileTransferSendStatus.Cancelled(
                    transferId = transferId,
                    sentBytes = 0uL,
                    totalBytes = totalBytes,
                ),
            )
            return
        }

        publish(
            AndroidFileTransferSendStatus.WaitingForPeer(
                transferId = transferId,
                totalBytes = totalBytes,
            ),
        )

        val acceptance =
            runCatching {
                agent.sendFileTransferOffer(prepared)
            }.getOrElse {
                fail(
                    transferId,
                    0,
                    totalBytes,
                    AndroidFileTransferSendFailure.CONNECTION,
                )
                return
            }

        if (acceptance.kind == MobileFileTransferAcceptanceKind.ALREADY_COMPLETE) {
            completeSuccess(token)
            return
        }

        if (cancelled.get() || closed.get()) {
            runCatching { agent.cancelFileTransferOffer(transferId) }
            publish(
                AndroidFileTransferSendStatus.Cancelled(
                    transferId = transferId,
                    sentBytes = 0uL,
                    totalBytes = totalBytes,
                ),
            )
            return
        }

        val stream =
            runCatching {
                agent.openFileTransferStream(transferId)
            }.getOrElse {
                fail(
                    transferId,
                    0,
                    totalBytes,
                    AndroidFileTransferSendFailure.CONNECTION,
                )
                return
            }

        val resumeOffset = stream.resumeOffset()
        val input = adapter.openSource(token.source)
        if (input == null) {
            runCatching { agent.cancelFileTransferSend(stream) }
            fail(
                transferId,
                resumeOffset,
                totalBytes,
                AndroidFileTransferSendFailure.SOURCE,
            )
            return
        }

        input.use { source ->
            if (!skipExactly(source, resumeOffset)) {
                runCatching { agent.cancelFileTransferSend(stream) }
                fail(
                    transferId,
                    resumeOffset,
                    totalBytes,
                    AndroidFileTransferSendFailure.SOURCE_CHANGED,
                )
                return
            }

            var sent = resumeOffset
            val buffer = ByteArray(FILE_TRANSFER_SEND_IO_BYTES)
            while (sent < totalBytes) {
                if (cancelled.get() || closed.get()) {
                    runCatching { agent.cancelFileTransferSend(stream) }
                    publish(
                        AndroidFileTransferSendStatus.Cancelled(
                            transferId = transferId,
                            sentBytes = sent,
                            totalBytes = totalBytes,
                        ),
                    )
                    return
                }

                val remaining = totalBytes - sent
                val requested =
                    minOf(
                        buffer.size.toULong(),
                        remaining,
                    ).toInt()
                val read =
                    runCatching {
                        source.read(buffer, 0, requested)
                    }.getOrElse {
                        runCatching { agent.cancelFileTransferSend(stream) }
                        fail(
                            transferId,
                            sent,
                            totalBytes,
                            AndroidFileTransferSendFailure.SOURCE,
                        )
                        return
                    }
                if (read < 0) {
                    runCatching { agent.cancelFileTransferSend(stream) }
                    fail(
                        transferId,
                        sent,
                        totalBytes,
                        AndroidFileTransferSendFailure.SOURCE_CHANGED,
                    )
                    return
                }
                if (read == 0) continue

                var chunk = buffer.copyOf(read)
                while (true) {
                    if (cancelled.get() || closed.get()) {
                        runCatching { agent.cancelFileTransferSend(stream) }
                        publish(
                            AndroidFileTransferSendStatus.Cancelled(
                                transferId = transferId,
                                sentBytes = sent,
                                totalBytes = totalBytes,
                            ),
                        )
                        return
                    }

                    val result =
                        runCatching {
                            agent.sendFileTransferChunk(stream, chunk)
                        }.getOrElse {
                            fail(
                                transferId,
                                sent,
                                totalBytes,
                                AndroidFileTransferSendFailure.CONNECTION,
                            )
                            return
                        }

                    when (result.outcome()) {
                        MobileFileTransferChunkOutcome.SENT -> break

                        MobileFileTransferChunkOutcome.BACKPRESSURE -> {
                            chunk = result.takeChunk() ?: run {
                                fail(
                                    transferId,
                                    sent,
                                    totalBytes,
                                    AndroidFileTransferSendFailure.CONNECTION,
                                )
                                return
                            }
                            try {
                                Thread.sleep(BACKPRESSURE_RETRY_MS)
                            } catch (_: InterruptedException) {
                                Thread.currentThread().interrupt()
                                return
                            }
                        }

                        MobileFileTransferChunkOutcome.TOO_LARGE -> {
                            fail(
                                transferId,
                                sent,
                                totalBytes,
                                AndroidFileTransferSendFailure.INTERNAL,
                            )
                            return
                        }

                        MobileFileTransferChunkOutcome.CLOSED -> {
                            val recovered =
                                runCatching {
                                    agent.recoverClosedFileTransferResult(stream)
                                }.getOrNull()
                            if (recovered != null) {
                                handleTerminal(token, recovered.outcome, sent)
                            } else {
                                fail(
                                    transferId,
                                    sent,
                                    totalBytes,
                                    AndroidFileTransferSendFailure.CONNECTION,
                                )
                            }
                            return
                        }
                    }
                }

                sent += read.toULong()
                publish(
                    AndroidFileTransferSendStatus.Sending(
                        transferId = transferId,
                        sentBytes = sent,
                        totalBytes = totalBytes,
                    ),
                )
            }

            if (source.read() >= 0) {
                runCatching { agent.cancelFileTransferSend(stream) }
                fail(
                    transferId,
                    sent,
                    totalBytes,
                    AndroidFileTransferSendFailure.SOURCE_CHANGED,
                )
                return
            }

            publish(
                AndroidFileTransferSendStatus.Finalizing(
                    transferId = transferId,
                    totalBytes = totalBytes,
                ),
            )
            val result =
                runCatching {
                    agent.finishFileTransferStream(stream)
                }.getOrElse {
                    val recovered =
                        runCatching {
                            agent.recoverClosedFileTransferResult(stream)
                        }.getOrNull()
                    if (recovered != null) {
                        handleTerminal(token, recovered.outcome, sent)
                    } else {
                        fail(
                            transferId,
                            sent,
                            totalBytes,
                            AndroidFileTransferSendFailure.CONNECTION,
                        )
                    }
                    return
                }
            handleTerminal(token, result.outcome, sent)
        }
    }

    private fun handleTerminal(
        token: RetryToken,
        outcome: MobileFileTransferTerminalOutcome,
        sent: ULong,
    ) {
        when (outcome) {
            MobileFileTransferTerminalOutcome.COMPLETED -> completeSuccess(token)

            MobileFileTransferTerminalOutcome.CANCELLED ->
                publish(
                    AndroidFileTransferSendStatus.Cancelled(
                        transferId = token.prepared.transferId(),
                        sentBytes = sent,
                        totalBytes = token.prepared.fileSize(),
                    ),
                )

            MobileFileTransferTerminalOutcome.INTEGRITY_FAILED ->
                fail(
                    token.prepared.transferId(),
                    sent,
                    token.prepared.fileSize(),
                    AndroidFileTransferSendFailure.INTEGRITY,
                )

            MobileFileTransferTerminalOutcome.STORAGE_FAILED ->
                fail(
                    token.prepared.transferId(),
                    sent,
                    token.prepared.fileSize(),
                    AndroidFileTransferSendFailure.STORAGE,
                )
        }
    }

    private fun completeSuccess(token: RetryToken) {
        synchronized(lock) {
            if (retryToken?.prepared?.transferId()?.contentEquals(token.prepared.transferId()) == true) {
                retryToken = null
            }
        }
        publish(
            AndroidFileTransferSendStatus.Completed(
                transferId = token.prepared.transferId(),
                totalBytes = token.prepared.fileSize(),
            ),
        )
    }

    private fun fail(
        transferId: ByteArray?,
        sentBytes: ULong,
        totalBytes: ULong?,
        failure: AndroidFileTransferSendFailure,
    ) {
        publish(
            AndroidFileTransferSendStatus.Failed(
                transferId = transferId,
                sentBytes = sentBytes,
                totalBytes = totalBytes,
                failure = failure,
            ),
        )
    }

    private fun publish(status: AndroidFileTransferSendStatus) {
        listeners.forEach { it(status) }
    }

    private data class RetryToken(
        val source: AndroidFileTransferSource,
        val prepared: MobilePreparedFileTransfer,
    )
}

internal fun skipExactly(
    input: InputStream,
    offset: ULong,
): Boolean {
    var remaining = offset
    val discard = ByteArray(FILE_TRANSFER_SEND_IO_BYTES)
    while (remaining > 0uL) {
        val skipped =
            runCatching {
                input.skip(minOf(remaining, Long.MAX_VALUE.toULong()).toLong())
            }.getOrElse { return false }
        if (skipped > 0) {
            remaining -= skipped.toULong()
            continue
        }

        val requested =
            minOf(
                remaining,
                discard.size.toULong(),
            ).toInt()
        val read =
            runCatching {
                input.read(discard, 0, requested)
            }.getOrElse { return false }
        if (read <= 0) return false
        remaining -= read.toULong()
    }
    return true
}
