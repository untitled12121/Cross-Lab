package dev.crosslab.android.features.filetransfer

import android.content.Context
import android.net.Uri
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobileFileTransferException
import uniffi.crosslab_mobile_ffi.MobileFileTransferDataEvent
import uniffi.crosslab_mobile_ffi.MobileFileTransferDataKind
import uniffi.crosslab_mobile_ffi.MobileFileTransferOffer
import uniffi.crosslab_mobile_ffi.MobileFileTransferTerminalOutcome
import uniffi.crosslab_mobile_ffi.MobileTrustedPresenceAgent

private const val WAIT_MS = 1_000uL

internal class IncomingOffer(
    val requestId: ByteArray,
    val sourceId: ByteArray,
    val offer: MobileFileTransferOffer,
    val canResume: Boolean,
)

internal class ActiveFileReceive(
    val request: IncomingOffer,
    val receiver: AndroidFileTransferReceiver,
    val offset: ULong,
) {
    var streamId: ByteArray? = null
    var transferred = offset

    fun matches(event: MobileFileTransferDataEvent): Boolean =
        event.transferId().contentEquals(request.offer.transferId()) &&
            (streamId == null || event.streamId().contentEquals(streamId))
}

class AndroidFileTransferService(context: Context) : FileTransferPort, AutoCloseable {
    override val fileTransferAvailable = true
    private val adapter = AndroidFileTransferAdapter(context)
    private val listeners = CopyOnWriteArraySet<(FileTransferState) -> Unit>()
    private val events = Executors.newFixedThreadPool(3) { task ->
        Thread(task, "crosslab-file-events").apply { isDaemon = true }
    }
    private val io = Executors.newFixedThreadPool(2) { task ->
        Thread(task, "crosslab-file-io").apply { isDaemon = true }
    }
    private val control = Executors.newSingleThreadExecutor { task ->
        Thread(task, "crosslab-file-control").apply { isDaemon = true }
    }
    private val closed = AtomicBoolean(false)
    private val lock = Any()

    @Volatile private var current = FileTransferState.initial(true)
    private var agent: MobileTrustedPresenceAgent? = null
    private var lifecycle: Long = 0
    private var session: Long = 0
    private var connected = false
    private var pending: IncomingOffer? = null
    private var preparing: IncomingOffer? = null
    private var receiving: ActiveFileReceive? = null
    private var sending: AndroidFileTransferSender? = null
    private var retry: FileSendSelection? = null

    override fun fileTransferState() = current

    override fun observeFileTransfer(listener: (FileTransferState) -> Unit): AutoCloseable {
        listeners += listener
        listener(current)
        return AutoCloseable { listeners -= listener }
    }

    fun attach(active: MobileTrustedPresenceAgent) {
        val token = synchronized(lock) {
            if (closed.get()) return
            detachLocked()
            agent = active
            lifecycle
        }
        events.execute { readOffers(active, token) }
        events.execute { readCancellations(active, token) }
        events.execute { readData(active, token) }
    }

    fun detach(active: MobileTrustedPresenceAgent) {
        synchronized(lock) {
            if (agent !== active) return
            detachLocked()
        }
    }

    fun sessionConnected(active: MobileTrustedPresenceAgent, online: Boolean) {
        synchronized(lock) {
            if (agent !== active || connected == online) return
            connected = online
            if (!online) interruptLocked()
        }
    }

    override fun sendFile(uri: Uri, retainReadGrant: Boolean): Boolean =
        beginSend(FileSendSelection(uri, null, retainReadGrant))

    override fun retryFile(): Boolean {
        val selection = synchronized(lock) {
            if (!current.canRetry || !connected) return false
            retry
        } ?: return false
        return beginSend(selection)
    }

    override fun acceptFile(uri: Uri, requestId: String, retainWriteGrant: Boolean): Boolean =
        beginReceive(uri, requestId, retainWriteGrant)

    override fun resumeFile(requestId: String): Boolean =
        beginReceive(null, requestId, false)

    private fun beginReceive(
        uri: Uri?,
        requestId: String,
        retainWriteGrant: Boolean,
    ): Boolean {
        val input: ReceiveSelection
        synchronized(lock) {
            val active = agent ?: return false
            if (!connected || preparing != null || receiving != null) return false
            val request = pending ?: return false
            if (request.requestId.toHex() != requestId ||
                (uri == null && !request.canResume)) return false
            pending = null
            preparing = request
            input = ReceiveSelection(active, session, request, uri, retainWriteGrant)
            publishLocked(state(request.offer, FileTransferDirection.RECEIVE, FileTransferStage.PREPARING))
        }
        io.execute { prepareReceive(input) }
        return true
    }

    override fun declineFile(): Boolean {
        val target: Pair<MobileTrustedPresenceAgent, ByteArray>
        synchronized(lock) {
            val active = agent ?: return false
            val request = pending ?: return false
            pending = null
            target = active to request.requestId
            publishLocked(state(request.offer, FileTransferDirection.RECEIVE, FileTransferStage.CANCELLED))
        }
        control.execute { runCatching { target.first.declineFileTransferRequest(target.second) } }
        return true
    }

    override fun cancelFile(): Boolean {
        val cancellation: () -> Unit
        synchronized(lock) {
            val active = agent ?: return false
            val sender = sending
            val request = pending ?: preparing
            val receive = receiving
            when {
                sender != null && current.canCancel -> {
                    sender.cancelled.set(true)
                    publishLocked(state(sender.offer, FileTransferDirection.SEND,
                        FileTransferStage.CANCELLED, canRetry = retry != null))
                    cancellation = { sender.cancel() }
                }
                request != null -> {
                    pending = null
                    preparing = null
                    publishLocked(state(request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.CANCELLED))
                    cancellation = { runCatching { active.declineFileTransferRequest(request.requestId) } }
                }
                receive != null && receive.receiver.requestCancellation() -> {
                    receiving = null
                    publishLocked(state(receive.request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.CANCELLED, receive.transferred))
                    cancellation = {
                        runCatching { active.cancelFileTransferReceive(receive.request.offer.transferId()) }
                        receive.receiver.cancel()
                    }
                }
                else -> return false
            }
        }
        control.execute { cancellation() }
        return true
    }

    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        synchronized(lock) { detachLocked() }
        events.shutdownNow()
        io.shutdownNow()
        control.shutdownNow()
        listeners.clear()
    }

    private fun beginSend(selection: FileSendSelection): Boolean {
        val active: MobileTrustedPresenceAgent
        val worker: AndroidFileTransferSender
        val token: Long
        synchronized(lock) {
            active = agent ?: return false
            if (!connected || sending != null || pending != null ||
                preparing != null || receiving != null) return false
            token = session
            if (selection.originalOffer == null) retry = null
            worker = AndroidFileTransferSender(adapter, active, selection) { progress, prepared ->
                synchronized(lock) {
                    if (!currentSession(active, token) || sending?.selection !== selection ||
                        sending?.cancelled?.get() == true) {
                        return@synchronized
                    }
                    if (prepared != null) {
                        retry = FileSendSelection(selection.uri, prepared, selection.retainGrant)
                    }
                    publishLocked(progress)
                }
            }
            sending = worker
            publishLocked(state(selection.originalOffer, FileTransferDirection.SEND,
                FileTransferStage.PREPARING, canCancel = true))
        }
        io.execute {
            val result = worker.run()
            synchronized(lock) {
                if (sending !== worker || !currentSession(active, token)) return@synchronized
                sending = null
                if (result.stage == FileTransferStage.COMPLETED ||
                    result.failure == FileTransferFailure.SOURCE_CHANGED) retry = null
                val offer = worker.offer
                publishLocked(state(offer, FileTransferDirection.SEND, result.stage,
                    result.transferred, canRetry = retry != null &&
                        result.stage != FileTransferStage.COMPLETED, failure = result.failure))
            }
        }
        return true
    }

    private fun prepareReceive(input: ReceiveSelection) {
        var prepared: AndroidReceivePreparation? = null
        try {
            val preparation = adapter.prepareReceive(
                input.request.sourceId, input.request.offer, input.uri, input.retainGrant,
            )
            prepared = preparation
            synchronized(lock) {
                if (!currentSession(input.agent, input.token) || preparing !== input.request) {
                    preparation.receiver?.cancel()
                    return
                }
                preparing = null
                if (preparation.kind == AndroidReceivePreparationKind.READY) {
                    receiving = ActiveFileReceive(input.request, checkNotNull(preparation.receiver),
                        preparation.resumeOffset)
                    publishLocked(state(input.request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.WAITING_PEER, preparation.resumeOffset, canCancel = true))
                }
            }
            if (preparation.kind == AndroidReceivePreparationKind.ALREADY_COMPLETE) {
                input.agent.completeFileTransferAlreadyComplete(input.request.requestId)
                synchronized(lock) {
                    if (currentSession(input.agent, input.token) &&
                        current.direction == FileTransferDirection.RECEIVE &&
                        current.stage == FileTransferStage.PREPARING) {
                        publishLocked(state(input.request.offer, FileTransferDirection.RECEIVE,
                            FileTransferStage.COMPLETED, input.request.offer.fileSize()))
                    }
                }
            } else {
                input.agent.completeFileTransferReady(input.request.requestId, preparation.resumeOffset)
            }
        } catch (error: Throwable) {
            prepared?.receiver?.cancel()
            synchronized(lock) {
                if (currentSession(input.agent, input.token) &&
                    (preparing === input.request || receiving?.request === input.request)) {
                    preparing = null
                    if (receiving?.request === input.request) receiving = null
                    val cancelled = error is MobileFileTransferException.Cancelled
                    publishLocked(state(
                        input.request.offer,
                        FileTransferDirection.RECEIVE,
                        if (cancelled) FileTransferStage.CANCELLED else FileTransferStage.FAILED,
                        failure = if (cancelled) null else
                            fileTransferFailure(error, FileTransferFailure.STORAGE),
                    ))
                }
            }
            runCatching { input.agent.declineFileTransferRequest(input.request.requestId) }
        }
    }

    private fun readOffers(active: MobileTrustedPresenceAgent, token: Long) {
        while (isAgent(active, token)) {
            val incoming = try {
                active.waitFileTransferRequest(WAIT_MS)
            } catch (_: Throwable) { return } ?: continue
            val sourceId = incoming.sourceDeviceId()
            val offer = incoming.offer()
            val resumeAvailable =
                runCatching { adapter.canResume(sourceId, offer) }.getOrDefault(false)
            val request = IncomingOffer(incoming.requestId(), sourceId, offer, resumeAvailable)
            val accept = synchronized(lock) {
                if (!isAgentLocked(active, token)) return
                if (!connected || pending != null || preparing != null ||
                    receiving != null || sending != null) false
                else {
                    pending = request
                    publishLocked(state(request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.WAITING_DESTINATION, canCancel = true,
                        pendingRequestId = request.requestId.toHex(),
                        resumeAvailable = request.canResume))
                    true
                }
            }
            if (!accept) control.execute {
                runCatching { active.declineFileTransferRequest(request.requestId) }
            }
        }
    }

    private fun readCancellations(active: MobileTrustedPresenceAgent, token: Long) {
        while (isAgent(active, token)) {
            val event = try {
                active.waitFileTransferCancellation(WAIT_MS)
            } catch (_: Throwable) { return } ?: continue
            val id = event.transferId()
            synchronized(lock) {
                if (!isAgentLocked(active, token)) return
                val request = pending ?: preparing
                if (request != null && request.offer.transferId().contentEquals(id)) {
                    pending = null
                    preparing = null
                    publishLocked(state(request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.CANCELLED))
                }
                val received = receiving
                if (received != null && received.request.offer.transferId().contentEquals(id)) {
                    received.receiver.requestCancellation()
                    receiving = null
                    io.execute { received.receiver.cancel() }
                    publishLocked(state(received.request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.CANCELLED, received.transferred))
                }
            }
        }
    }

    private fun readData(active: MobileTrustedPresenceAgent, token: Long) {
        while (isAgent(active, token)) {
            val event = try {
                active.waitFileTransferData(WAIT_MS)
            } catch (_: Throwable) { return } ?: continue
            dispatchData(active, token, event)
        }
    }

    private fun dispatchData(
        active: MobileTrustedPresenceAgent,
        token: Long,
        event: MobileFileTransferDataEvent,
    ) {
        val received = synchronized(lock) {
            if (!isAgentLocked(active, token)) return
            receiving?.takeIf { it.matches(event) }
        } ?: return
        when (event.kind()) {
            MobileFileTransferDataKind.OPENED -> {
                val offset = event.resumeOffset()
                if (received.streamId != null || offset != received.offset) {
                    runCatching {
                        active.failFileTransferReceive(event.streamId(),
                            received.request.offer.transferId(),
                            MobileFileTransferTerminalOutcome.STORAGE_FAILED)
                    }
                    failReceive(active, received, FileTransferFailure.STORAGE)
                    return
                }
                synchronized(lock) {
                    if (receiving !== received) return
                    received.streamId = event.streamId()
                    publishLocked(state(received.request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.TRANSFERRING, received.transferred, canCancel = true))
                }
            }
            MobileFileTransferDataKind.CHUNK -> {
                if (received.streamId == null) return
                try {
                    val bytes = event.takeChunk() ?: return
                    val transferred = received.receiver.write(bytes)
                    synchronized(lock) {
                        if (receiving !== received) return
                        received.transferred = transferred
                        publishLocked(state(received.request.offer, FileTransferDirection.RECEIVE,
                            FileTransferStage.TRANSFERRING, transferred, canCancel = true))
                    }
                } catch (_: FileTransferPreparationCancelled) {
                    // Owner cancellation already revoked this receiver.
                } catch (_: FileTransferIntegrityFailure) {
                    failReceive(active, received, FileTransferFailure.INTEGRITY)
                } catch (_: Throwable) {
                    failReceive(active, received, FileTransferFailure.STORAGE)
                }
            }
            MobileFileTransferDataKind.FINISHED -> {
                if (received.streamId == null) return
                synchronized(lock) {
                    if (receiving !== received) return
                    publishLocked(state(received.request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.FINALIZING, received.transferred, canCancel = true))
                }
                try {
                    received.receiver.finish {
                        synchronized(lock) {
                            if (receiving === received) {
                                publishLocked(state(received.request.offer,
                                    FileTransferDirection.RECEIVE, FileTransferStage.FINALIZING,
                                    received.transferred, canCancel = false))
                            }
                        }
                    }
                    // Publication is durable even if the peer loses the terminal acknowledgement.
                    runCatching {
                        active.completeFileTransferResult(received.request.offer.transferId(),
                            MobileFileTransferTerminalOutcome.COMPLETED)
                    }
                    synchronized(lock) {
                        if (receiving !== received) return
                        receiving = null
                        publishLocked(state(received.request.offer, FileTransferDirection.RECEIVE,
                            FileTransferStage.COMPLETED, received.request.offer.fileSize()))
                    }
                } catch (_: FileTransferPreparationCancelled) {
                    // Cancel action sends the typed terminal result.
                } catch (_: FileTransferIntegrityFailure) {
                    failReceive(active, received, FileTransferFailure.INTEGRITY)
                } catch (_: Throwable) {
                    failReceive(active, received, FileTransferFailure.STORAGE)
                }
            }
            MobileFileTransferDataKind.CANCELLED -> {
                synchronized(lock) {
                    if (receiving !== received) return
                    receiving = null
                    received.receiver.requestCancellation()
                    publishLocked(state(received.request.offer, FileTransferDirection.RECEIVE,
                        FileTransferStage.CANCELLED, received.transferred))
                }
                io.execute { received.receiver.cancel() }
            }
        }
    }

    private fun failReceive(
        active: MobileTrustedPresenceAgent,
        received: ActiveFileReceive,
        reason: FileTransferFailure,
    ) {
        val stream = received.streamId
        if (stream != null) control.execute {
            runCatching {
                active.failFileTransferReceive(stream, received.request.offer.transferId(),
                    if (reason == FileTransferFailure.INTEGRITY)
                        MobileFileTransferTerminalOutcome.INTEGRITY_FAILED
                    else MobileFileTransferTerminalOutcome.STORAGE_FAILED)
            }
        }
        synchronized(lock) {
            if (receiving !== received) return
            receiving = null
            publishLocked(state(received.request.offer, FileTransferDirection.RECEIVE,
                FileTransferStage.FAILED, received.transferred, failure = reason))
        }
        io.execute { received.receiver.cancel() }
    }

    private fun isAgent(active: MobileTrustedPresenceAgent, token: Long) =
        synchronized(lock) { isAgentLocked(active, token) }

    private fun isAgentLocked(active: MobileTrustedPresenceAgent, token: Long) =
        !closed.get() && agent === active && lifecycle == token

    private fun currentSession(active: MobileTrustedPresenceAgent, token: Long) =
        agent === active && session == token && connected

    private fun detachLocked() {
        lifecycle++
        connected = false
        interruptLocked()
        agent = null
        retry = null
        publishLocked(FileTransferState.initial(true))
    }

    private fun interruptLocked() {
        session++
        val oldSend = sending
        val oldReceive = receiving
        oldSend?.cancelled?.set(true)
        sending = null
        pending = null
        preparing = null
        receiving = null
        if (oldReceive != null) io.execute { oldReceive.receiver.cancel() }
        if (oldSend != null || oldReceive != null) {
            val offer = oldSend?.offer ?: oldReceive?.request?.offer
            publishLocked(state(offer,
                if (oldSend != null) FileTransferDirection.SEND else FileTransferDirection.RECEIVE,
                FileTransferStage.FAILED, failure = FileTransferFailure.NOT_CONNECTED,
                canRetry = oldSend != null && retry != null))
        }
    }

    private fun publishLocked(state: FileTransferState) {
        current = state
        listeners.forEach { it(state) }
    }

    private fun state(
        offer: MobileFileTransferOffer?,
        direction: FileTransferDirection,
        stage: FileTransferStage,
        transferred: ULong = 0uL,
        canCancel: Boolean = false,
        canRetry: Boolean = false,
        pendingRequestId: String? = null,
        resumeAvailable: Boolean = false,
        failure: FileTransferFailure? = null,
    ) = FileTransferState(
        available = true, direction = direction, stage = stage,
        displayName = offer?.displayName(), transferredBytes = transferred,
        totalBytes = offer?.fileSize() ?: 0uL, canCancel = canCancel,
        canRetry = canRetry, pendingRequestId = pendingRequestId,
        resumeAvailable = resumeAvailable, failure = failure,
    )

    private class ReceiveSelection(
        val agent: MobileTrustedPresenceAgent,
        val token: Long,
        val request: IncomingOffer,
        val uri: Uri?,
        val retainGrant: Boolean,
    )
}
