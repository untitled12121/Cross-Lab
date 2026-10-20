package dev.crosslab.android

import android.os.Bundle
import android.content.Intent
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.activity.compose.setContent
import dev.crosslab.android.features.audit.OwnerAuditAction
import dev.crosslab.android.features.audit.OwnerAuditActionGate
import dev.crosslab.android.features.appearance.ThemeId
import uniffi.crosslab_mobile_ffi.MobileAuditAction
import uniffi.crosslab_mobile_ffi.MobileAuditOutcome
import dev.crosslab.android.features.clipboard.ClipboardState
import dev.crosslab.android.features.appearance.ThemeParser
import dev.crosslab.android.features.appearance.ThemeResolver
import dev.crosslab.android.features.controlcenter.ControlCenterScreen
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.features.devices.NativeSessionBinding
import dev.crosslab.android.features.filetransfer.FileTransferState
import dev.crosslab.android.features.identity.revocationTarget
import dev.crosslab.android.features.notifications.NotificationOwnerConsent
import dev.crosslab.android.features.observers.UiCallbackGate
import dev.crosslab.android.features.observers.closeObservers
import dev.crosslab.android.features.devices.RuntimeLifecycle
import dev.crosslab.android.features.devices.RuntimePermissionEffect
import dev.crosslab.android.features.pairing.PairingJoinerStage
import dev.crosslab.android.features.pairing.PairingJoinerState
import dev.crosslab.android.features.owner.OwnerWorkQueue

class MainActivity : ComponentActivity() {
    private val runtimeState = mutableStateOf(RuntimeControllerState.initial())
    private val clipboardState = mutableStateOf(ClipboardState.initial(false))
    private val fileTransferState = mutableStateOf(FileTransferState.initial(false))
    private val pairingState = mutableStateOf(PairingJoinerState.idle())
    private val trustedPeerIds = mutableStateOf<List<String>>(emptyList())
    private val trustedPeerDeviceIds = mutableStateOf<List<ByteArray>>(emptyList())
    private val trustedPeerGeneration = mutableIntStateOf(0)
    private val revokedPeerIds = mutableStateOf<List<String>>(emptyList())
    private val revocationNotice = mutableStateOf<String?>(null)
    private val auditRows = mutableStateOf<List<String>>(emptyList())
    private val auditDropped = mutableStateOf(0L)
    private val auditNotice = mutableStateOf<String?>(null)
    private val auditUnavailable = mutableStateOf(false)
    private val auditActionBusy = mutableStateOf(false)
    private val auditActionGate = OwnerAuditActionGate()
    private var pendingAuditExportTicket: Long? = null
    private val notificationConsent =
        mutableStateOf(NotificationOwnerConsent(false, false, false, false, false))
    private val notificationNotice = mutableStateOf<String?>(null)
    private val peerPermissionNotice = mutableStateOf<String?>(null)
    private val peerPermissionBusy = mutableStateOf(false)
    private val identityWorker = OwnerWorkQueue()
    private val uiObservationGate = UiCallbackGate { callback -> runOnUiThread { callback() } }
    private var runtimeSubscription: AutoCloseable? = null
    private var auditSubscription: AutoCloseable? = null
    private var clipboardSubscription: AutoCloseable? = null
    private var fileTransferSubscription: AutoCloseable? = null
    private var pairingSubscription: AutoCloseable? = null
    private var pendingFileSourceSession: NativeSessionBinding? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val app = application as CrossLabApplication
        refreshTrustedPeers(app)
        auditSubscription = app.auditRecorder.observe { status ->
            uiObservationGate.dispatch {
                auditUnavailable.value = status.unavailable
                if (status.unavailable) {
                    auditRows.value = emptyList()
                    auditDropped.value = 0L
                } else {
                    status.history?.let { updateAudit(it) }
                }
                auditNotice.value =
                    when {
                        status.unavailable -> "Audit write unavailable; recent events may be missing."
                        status.droppedEvents > 0 ->
                            "${status.droppedEvents} recent audit events were dropped."
                        else -> auditNotice.value
                    }
            }
        }
        refreshNotificationConsent(app)
        runtimeState.value = app.runtimeController.state()
        runtimeSubscription =
            app.runtimeController.observe { state ->
                uiObservationGate.dispatch { runtimeState.value = state }
            }
        clipboardState.value = app.clipboardController.state()
        clipboardSubscription =
            app.clipboardController.observe { state ->
                uiObservationGate.dispatch { clipboardState.value = state }
            }
        fileTransferState.value = app.fileTransferController.state()
        fileTransferSubscription =
            app.fileTransferController.observe { state ->
                uiObservationGate.dispatch { fileTransferState.value = state }
            }
        pairingState.value = app.pairingController.state()
        pairingSubscription =
            app.pairingController.observe { state ->
                uiObservationGate.dispatch {
                    pairingState.value = state
                    if (state.stage == PairingJoinerStage.PAIRED) {
                        refreshTrustedPeers(app)
                    }
                }
            }

        val theme =
            assets.open(ThemeResolver.assetName(ThemeId.AYU_LIGHT))
                .bufferedReader()
                .use { ThemeParser.parse(it.readText()) }

        setContent {
            var retainFileAccess by remember { mutableStateOf(false) }
            var pendingReceiveRequest by remember { mutableStateOf<String?>(null) }
            var reauthorizeRequest by remember { mutableStateOf<String?>(null) }
            val chooseSource =
                rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
                    val expected = pendingFileSourceSession
                    pendingFileSourceSession = null
                    if (uri != null &&
                        expected?.matches(app.runtimeController.sessionBinding("files.transfer")) == true
                    ) {
                        app.fileTransferController.send(uri, retainFileAccess)
                    }
                }
            val chooseDestination =
                rememberLauncherForActivityResult(
                    ActivityResultContracts.CreateDocument("application/octet-stream"),
                ) { uri ->
                    val requestId = pendingReceiveRequest
                    pendingReceiveRequest = null
                    if (uri != null && requestId != null) {
                        app.fileTransferController.accept(uri, requestId, retainFileAccess)
                    }
                }
            val reauthorizeDocument =
                rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
                    val requestId = reauthorizeRequest
                    reauthorizeRequest = null
                    if (uri != null && requestId != null) {
                        app.fileTransferController.accept(uri, requestId, retainFileAccess)
                    }
                }
            val exportAudit =
                rememberLauncherForActivityResult(
                    ActivityResultContracts.CreateDocument("text/csv"),
                ) { uri ->
                    val ticket = pendingAuditExportTicket
                    pendingAuditExportTicket = null
                    if (ticket != null) {
                        if (uri == null) {
                            finishAuditAction(ticket, "Audit export cancelled.")
                        } else {
                            val submitted = runCatching {
                                app.auditRecorder.export { csv ->
                                    if (csv.isFailure) {
                                        finishAuditAction(ticket, "Protected audit export unavailable.")
                                    } else if (!identityWorker.submit {
                                        val result = runCatching {
                                            val bytes = checkNotNull(csv.getOrNull())
                                                .toByteArray(Charsets.UTF_8)
                                            contentResolver.openOutputStream(uri)?.use { it.write(bytes) }
                                                ?: error("export output unavailable")
                                        }
                                        finishAuditAction(
                                            ticket,
                                            if (result.isSuccess) "Redacted history exported."
                                            else "Could not write audit export to the selected document.",
                                        )
                                    }) {
                                        finishAuditAction(ticket, "Owner operations are busy. Retry audit export.")
                                    }
                                }
                            }
                            if (submitted.isFailure) {
                                finishAuditAction(ticket, "Protected audit export could not start.")
                            }
                        }
                    }
                }
            ControlCenterScreen(
                theme = theme,
                runtime = runtimeState.value,
                clipboard = clipboardState.value,
                fileTransfer = fileTransferState.value,
                retainFileAccess = retainFileAccess,
                onToggleRetainFileAccess = { retainFileAccess = !retainFileAccess },
                onSendFile = {
                    val binding = app.runtimeController.sessionBinding("files.transfer")
                    if (binding != null) {
                        pendingFileSourceSession = binding
                        chooseSource.launch(arrayOf("*/*"))
                    }
                },
                onChooseFileDestination = { name, id ->
                    pendingReceiveRequest = id
                    chooseDestination.launch(name)
                },
                onResumeFile = { id -> app.fileTransferController.resume(id) },
                onReauthorizeFile = { id ->
                    reauthorizeRequest = id
                    reauthorizeDocument.launch(arrayOf("*/*"))
                },
                onCancelFile = { app.fileTransferController.cancel() },
                onDeclineFile = { app.fileTransferController.decline() },
                onRetryFile = { app.fileTransferController.retry() },
                onDisconnect = { app.runtimeController.disconnectPeer() },
                onReconnect = { app.runtimeController.reconnectPeer() },
                onSendClipboard = app.clipboardController::send,
                onFetchClipboard = app.clipboardController::fetch,
                pairing = pairingState.value,
                trustedPeerIds = trustedPeerIds.value,
                inventoryGeneration = trustedPeerGeneration.intValue,
                revokedPeerIds = revokedPeerIds.value,
                canRevokePeers = app.identityRepository.canRevokePeers,
                revocationNotice = revocationNotice.value,
                auditRows = auditRows.value,
                auditDropped = auditDropped.value,
                auditNotice = auditNotice.value,
                auditUnavailable = auditUnavailable.value,
                auditActionBusy = auditActionBusy.value,
                onRetryAudit = {
                    val ticket = auditActionGate.begin(OwnerAuditAction.REFRESH)
                    if (ticket == null) {
                        auditNotice.value = "An audit operation is already in progress."
                    } else {
                        auditActionBusy.value = true
                        val result = runCatching {
                            app.auditRecorder.refresh { history ->
                                finishAuditAction(
                                    ticket,
                                    if (history.isSuccess) "Protected audit history reloaded."
                                    else "Audit storage remains unavailable; history is hidden.",
                                    history.getOrNull(),
                                    protectedFailure = history.isFailure,
                                )
                            }
                        }
                        if (result.isFailure) {
                            finishAuditAction(ticket, "Protected audit history could not be reloaded.", protectedFailure = true)
                        }
                    }
                },
                onClearAudit = {
                    val ticket = if (auditUnavailable.value) null
                        else auditActionGate.begin(OwnerAuditAction.CLEAR)
                    if (ticket == null) {
                        auditNotice.value = "Audit clear is unavailable while storage is offline or busy."
                    } else {
                        auditActionBusy.value = true
                        val submitted = runCatching {
                            app.auditRecorder.clear { result ->
                                finishAuditAction(
                                    ticket,
                                    if (result.isSuccess) "Local audit history cleared."
                                    else "Audit history could not be cleared.",
                                    result.getOrNull(),
                                    protectedFailure = result.isFailure,
                                )
                            }
                        }
                        if (submitted.isFailure) {
                            finishAuditAction(ticket, "Audit history could not be cleared.", protectedFailure = true)
                        }
                    }
                },
                onExportAudit = {
                    val ticket = if (auditUnavailable.value) null
                        else auditActionGate.begin(OwnerAuditAction.EXPORT)
                    if (ticket == null) {
                        auditNotice.value = "Audit export is unavailable while storage is offline or busy."
                    } else {
                        auditActionBusy.value = true
                        pendingAuditExportTicket = ticket
                        if (runCatching { exportAudit.launch("crosslab-audit.csv") }.isFailure) {
                            pendingAuditExportTicket = null
                            finishAuditAction(ticket, "Could not open the audit export document picker.")
                        }
                    }
                },
                notificationConsent = notificationConsent.value,
                onToggleNotificationOwner = {
                    val saved =
                        app.notificationConsent.setOwnerEnabled(!app.notificationConsent.ownerEnabled)
                    app.notificationPermissionChanged()
                    notificationNotice.value =
                        if (saved) null else "Owner consent could not be saved; mirroring is disabled."
                    refreshNotificationConsent(app)
                },
                onToggleNotificationContent = {
                    val saved =
                        app.notificationConsent.setContentEnabled(!app.notificationConsent.contentEnabled)
                    app.notificationPermissionChanged()
                    notificationNotice.value =
                        if (saved) null else "Content consent could not be saved; mirroring is disabled."
                    refreshNotificationConsent(app)
                },
                onOpenNotificationAccess = {
                    startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
                },
                notificationNotice = notificationNotice.value,
                peerPermissionNotice = peerPermissionNotice.value,
                peerPermissionBusy = peerPermissionBusy.value,
                onSetPeerPermission = { expectedPeer, capabilityId, operation, effect ->
                    if (!peerPermissionBusy.value) {
                        peerPermissionBusy.value = true
                        peerPermissionNotice.value = null
                        if (!identityWorker.submit {
                            val permitted = runCatching {
                                app.runtimeController.setPermissionForPeer(
                                    expectedPeer,
                                    capabilityId,
                                    operation,
                                    effect,
                                )
                            }
                            if (permitted.getOrDefault(false)) {
                                val revision = runCatching {
                                    app.policyStore.load()?.anchorRevision()
                                }.getOrNull()
                                if (revision != null) {
                                    app.auditRecorder.permissionCommitted(revision)
                                } else {
                                    app.auditRecorder.record(
                                        MobileAuditAction.PERMISSION_CHANGED,
                                        MobileAuditOutcome.SUCCEEDED,
                                    )
                                }
                            }
                            uiObservationGate.dispatch {
                                peerPermissionBusy.value = false
                                peerPermissionNotice.value =
                                    if (permitted.getOrDefault(false)) {
                                        "Device policy saved; new operations use the updated rule."
                                    } else {
                                        "Policy was not changed. Verify the current trusted session."
                                    }
                            }
                        }) {
                            peerPermissionBusy.value = false
                            peerPermissionNotice.value = "Owner operations are busy. Retry permission change."
                        }
                    }
                },
                onRevokePeer = { index, generation ->
                    val id = revocationTarget(
                        trustedPeerDeviceIds.value,
                        index,
                        generation,
                        trustedPeerGeneration.intValue,
                    )
                    if (id != null) {
                        if (!identityWorker.submit {
                            val result = runCatching {
                                app.identityRepository.revokePeer(id)
                                // Tear down the prior session before best-effort history I/O.
                                val reloaded = runCatching {
                                    app.runtimeController.reloadTrust()
                                }.getOrDefault(false)
                                app.auditRecorder.record(
                                    MobileAuditAction.PEER_REVOKED,
                                    MobileAuditOutcome.SUCCEEDED,
                                ) { savedAudit ->
                                    if (!isDestroyed && savedAudit.isFailure) runOnUiThread {
                                        if (!isDestroyed) {
                                            auditNotice.value =
                                                "Device revoked; audit history unavailable."
                                        }
                                    }
                                }
                                check(reloaded) {
                                    "Device revoked, but session restart failed. Restart Cross-Lab."
                                }
                            }
                            val identity = runCatching {
                                app.identityRepository.load()
                            }.getOrNull()
                            uiObservationGate.dispatch {
                                updateTrustedPeerState(identity)
                                revocationNotice.value =
                                    result.fold(
                                        onSuccess = { "Trust revoked and saved." },
                                        onFailure = { "Revocation could not complete. Check device status and secure storage." },
                                    )
                            }
                        }) {
                            revocationNotice.value = "Owner operations are busy. Retry revocation."
                        }
                    }
                },
                onPairingBootstrapScanned = app.pairingController::begin,
                onCancelPairing = app.pairingController::cancel,
            )
        }
    }

    override fun onResume() {
        super.onResume()
        (application as? CrossLabApplication)?.let(::refreshNotificationConsent)
    }

    private fun refreshNotificationConsent(app: CrossLabApplication) {
        notificationConsent.value = app.notificationConsent.current(
            app.runtimeController.state().lifecycle == RuntimeLifecycle.RUNNING,
        )
    }

    private fun finishAuditAction(
        ticket: Long,
        message: String,
        history: uniffi.crosslab_mobile_ffi.MobileAuditHistory? = null,
        protectedFailure: Boolean = false,
    ) {
        uiObservationGate.dispatch {
            if (auditActionGate.finish(ticket)) {
                auditActionBusy.value = false
                auditNotice.value = message
                if (protectedFailure) {
                    auditUnavailable.value = true
                    auditRows.value = emptyList()
                    auditDropped.value = 0L
                } else if (history != null) {
                    auditUnavailable.value = false
                    updateAudit(history)
                }
            }
        }
    }

    private fun updateAudit(history: uniffi.crosslab_mobile_ffi.MobileAuditHistory) {
        auditRows.value = history.rows
        auditDropped.value = history.droppedCount.toLong()
    }

    private fun refreshTrustedPeers(app: CrossLabApplication) {
        if (!identityWorker.submit {
            val identity = runCatching { app.identityRepository.load() }.getOrNull()
            uiObservationGate.dispatch { updateTrustedPeerState(identity) }
        }) {
            uiObservationGate.dispatch {
                revocationNotice.value = "Owner operations are busy. Retry device inventory refresh."
            }
        }
    }

    private fun updateTrustedPeerState(
        identity: uniffi.crosslab_mobile_ffi.MobileProductIdentity?,
    ) {
        trustedPeerIds.value = identity?.trustedPeerIds.orEmpty()
        trustedPeerDeviceIds.value = identity?.trustedPeerDeviceIds.orEmpty()
        revokedPeerIds.value = identity?.revokedPeerIds.orEmpty()
        trustedPeerGeneration.intValue += 1
    }

    override fun onDestroy() {
        // Block callbacks before tearing down subscriptions; one broken close cannot leak others.
        uiObservationGate.close()
        auditActionGate.close()
        pendingAuditExportTicket = null
        closeObservers(
            listOf(
                auditSubscription,
                runtimeSubscription,
                clipboardSubscription,
                fileTransferSubscription,
                pairingSubscription,
            ),
        )
        auditSubscription = null
        runtimeSubscription = null
        clipboardSubscription = null
        fileTransferSubscription = null
        pairingSubscription = null
        pendingFileSourceSession = null
        identityWorker.close()
        super.onDestroy()
    }
}
