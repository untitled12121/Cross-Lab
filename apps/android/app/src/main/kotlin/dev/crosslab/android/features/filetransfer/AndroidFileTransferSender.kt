package dev.crosslab.android.features.filetransfer

import android.net.Uri
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobileFileTransferAcceptanceKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferChunkOutcome
import uniffi.crosslab_mobile_ffi.MobileFileTransferException
import uniffi.crosslab_mobile_ffi.MobileFileTransferOffer
import uniffi.crosslab_mobile_ffi.MobileFileTransferSourceStream
import uniffi.crosslab_mobile_ffi.MobileFileTransferTerminalOutcome
import uniffi.crosslab_mobile_ffi.MobileTrustedPresenceAgent

private const val RETRY_BACKPRESSURE_MS = 10L

internal class FileSendSelection(
    val uri: Uri,
    val originalOffer: MobileFileTransferOffer?,
    val retainGrant: Boolean,
) {
    override fun toString(): String = "FileSendSelection(uri=[REDACTED])"
}

internal class AndroidFileTransferSender(
    private val adapter: AndroidFileTransferAdapter,
    private val agent: MobileTrustedPresenceAgent,
    val selection: FileSendSelection,
    private val onProgress: (FileTransferState, MobileFileTransferOffer?) -> Unit,
) {
    val cancelled = AtomicBoolean(false)

    @Volatile var offer: MobileFileTransferOffer? = selection.originalOffer
        private set

    @Volatile private var stream: MobileFileTransferSourceStream? = null

    fun cancel() {
        cancelled.set(true)
        val currentStream = stream
        if (currentStream != null) {
            runCatching { agent.cancelFileTransferSend(currentStream) }
        } else {
            offer?.let { runCatching { agent.cancelFileOffer(it.transferId()) } }
        }
    }

    fun run(): SendTerminal {
        var transferred = 0uL
        return try {
            if (selection.retainGrant) {
                adapter.persistUriPermission(selection.uri, read = true, write = false)
            }
            val source = adapter.prepareSource(
                selection.uri,
                selection.originalOffer,
                cancelled::get,
            )
            offer = source.offer
            if (cancelled.get()) return stopped(transferred)
            onProgress(status(FileTransferStage.WAITING_PEER, transferred, true), source.offer)

            val accepted = agent.sendFileOffer(source.offer)
            if (cancelled.get()) return stopped(transferred)
            if (accepted.kind() == MobileFileTransferAcceptanceKind.ALREADY_COMPLETE) {
                return SendTerminal(FileTransferStage.COMPLETED, source.offer.fileSize())
            }
            val offset = accepted.resumeOffset()
                ?: throw FileTransferStorageUnavailable("missing peer resume offset")
            transferred = offset
            adapter.openSource(source, offset).use { reader ->
                if (cancelled.get()) return stopped(transferred)
                val opened = agent.openFileTransferStream(source.offer.transferId())
                stream = opened
                if (cancelled.get()) return stopped(transferred)
                onProgress(status(FileTransferStage.TRANSFERRING, transferred, true), source.offer)

                while (!cancelled.get()) {
                    val bytes = reader.readChunk() ?: break
                    while (!cancelled.get()) {
                        when (agent.sendFileTransferChunk(opened, bytes)) {
                            MobileFileTransferChunkOutcome.SENT -> break
                            MobileFileTransferChunkOutcome.BACKPRESSURE ->
                                Thread.sleep(RETRY_BACKPRESSURE_MS)
                            MobileFileTransferChunkOutcome.TOO_LARGE ->
                                throw FileTransferStorageUnavailable("unexpected chunk size")
                            MobileFileTransferChunkOutcome.CLOSED ->
                                throw IllegalStateException("stream closed")
                        }
                    }
                    if (cancelled.get()) return stopped(transferred)
                    transferred += bytes.size.toULong()
                    onProgress(status(FileTransferStage.TRANSFERRING, transferred, true), source.offer)
                }
            }
            if (cancelled.get()) return stopped(transferred)
            onProgress(status(FileTransferStage.FINALIZING, transferred, false), source.offer)
            val result = agent.finishFileTransferStream(checkNotNull(stream))
            when (result.outcome()) {
                MobileFileTransferTerminalOutcome.COMPLETED ->
                    SendTerminal(FileTransferStage.COMPLETED, source.offer.fileSize())
                MobileFileTransferTerminalOutcome.CANCELLED -> stopped(transferred)
                MobileFileTransferTerminalOutcome.INTEGRITY_FAILED ->
                    SendTerminal(FileTransferStage.FAILED, transferred, FileTransferFailure.INTEGRITY)
                MobileFileTransferTerminalOutcome.STORAGE_FAILED ->
                    SendTerminal(FileTransferStage.FAILED, transferred, FileTransferFailure.STORAGE)
            }
        } catch (_: FileTransferPreparationCancelled) {
            stopped(transferred)
        } catch (_: FileTransferSourceChanged) {
            SendTerminal(FileTransferStage.FAILED, transferred, FileTransferFailure.SOURCE_CHANGED)
        } catch (_: FileTransferStorageUnavailable) {
            if (cancelled.get()) stopped(transferred)
            else SendTerminal(FileTransferStage.FAILED, transferred, FileTransferFailure.STORAGE)
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
            stopped(transferred)
        } catch (error: Throwable) {
            if (cancelled.get() || error is MobileFileTransferException.Cancelled) {
                stopped(transferred)
            } else {
                SendTerminal(
                    FileTransferStage.FAILED,
                    transferred,
                    fileTransferFailure(error, FileTransferFailure.NETWORK),
                )
            }
        } finally {
            stream = null
        }
    }

    private fun stopped(bytes: ULong): SendTerminal {
        // On cancel, no session-local stream authority is retained for a retry.
        cancel()
        return SendTerminal(FileTransferStage.CANCELLED, bytes)
    }

    private fun status(
        stage: FileTransferStage,
        transferred: ULong,
        canCancel: Boolean,
    ) = FileTransferState(
        available = true,
        direction = FileTransferDirection.SEND,
        stage = stage,
        displayName = offer?.displayName(),
        transferredBytes = transferred,
        totalBytes = offer?.fileSize() ?: 0uL,
        canCancel = canCancel,
    )
}

internal data class SendTerminal(
    val stage: FileTransferStage,
    val transferred: ULong,
    val failure: FileTransferFailure? = null,
)
