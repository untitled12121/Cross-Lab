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
import dev.crosslab.android.features.appearance.ThemeId
import uniffi.crosslab_mobile_ffi.MobileAuditAction
import uniffi.crosslab_mobile_ffi.MobileAuditOutcome
import dev.crosslab.android.features.clipboard.ClipboardState
import dev.crosslab.android.features.appearance.ThemeParser
import dev.crosslab.android.features.appearance.ThemeResolver
import dev.crosslab.android.features.controlcenter.ControlCenterScreen
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.features.devices.RuntimeSession
import dev.crosslab.android.features.filetransfer.FileTransferState
import dev.crosslab.android.features.identity.revocationTarget
import dev.crosslab.android.features.notifications.NotificationOwnerConsent
import dev.crosslab.android.features.devices.RuntimeLifecycle
import dev.crosslab.android.features.devices.RuntimePermissionEffect
import dev.crosslab.android.features.pairing.PairingJoinerStage
import dev.crosslab.android.features.pairing.PairingJoinerState
import java.util.concurrent.Executors

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
    private val notificationConsent =
        mutableStateOf(NotificationOwnerConsent(false, false, false, false, false))
    private val notificationNotice = mutableStateOf<String?>(null)
    private val identityWorker = Executors.newSingleThreadExecutor()
    private var runtimeSubscription: AutoCloseable? = null
    private var auditSubscription: AutoCloseable? = null
    private var clipboardSubscription: AutoCloseable? = null
    private var fileTransferSubscription: AutoCloseable? = null
    private var pairingSubscription: AutoCloseable? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val app = application as CrossLabApplication
        refreshTrustedPeers(app)
        auditSubscription = app.auditRecorder.observe { status ->
            if (!isDestroyed) runOnUiThread {
                if (!isDestroyed) {
                    status.history?.let { updateAudit(it) }
                    auditNotice.value =
                        when {
                            status.unavailable -> "Audit write unavailable; recent events may be missing."
                            status.droppedEvents > 0 ->
                                "${status.droppedEvents} recent audit events were dropped."
                            else -> auditNotice.value
                        }
                }
            }
        }
        refreshNotificationConsent(app)
        runtimeState.value = app.runtimeController.state()
        runtimeSubscription =
            app.runtimeController.observe { state ->
                runOnUiThread { runtimeState.value = state }
            }
        clipboardState.value = app.clipboardController.state()
        clipboardSubscription =
            app.clipboardController.observe { state ->
                runOnUiThread { clipboardState.value = state }
            }
        fileTransferState.value = app.fileTransferController.state()
        fileTransferSubscription =
            app.fileTransferController.observe { state ->
                runOnUiThread { fileTransferState.value = state }
            }
        pairingState.value = app.pairingController.state()
        pairingSubscription =
            app.pairingController.observe { state ->
                runOnUiThread { pairingState.value = state }
                if (state.stage == PairingJoinerStage.PAIRED) {
                    refreshTrustedPeers(app)
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
            var selectedPeer by remember { mutableStateOf<String?>(null) }
            val chooseSource =
                rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
                    val originalPeer = selectedPeer
                    selectedPeer = null
                    if (uri != null && originalPeer != null &&
                        runtimeState.value.snapshot.peerDeviceId == originalPeer &&
                        runtimeState.value.snapshot.session == RuntimeSession.ACTIVE
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
                    if (uri != null) {
                        app.auditRecorder.export { csv ->
                            identityWorker.execute {
                                val result = csv.mapCatching { text ->
                                    val bytes = text.toByteArray(Charsets.UTF_8)
                                    contentResolver.openOutputStream(uri)?.use { it.write(bytes) }
                                        ?: error("export output unavailable")
                                }
                                if (!isDestroyed) runOnUiThread {
                                    if (!isDestroyed) {
                                        auditNotice.value =
                                            if (result.isSuccess) {
                                                "Redacted history exported."
                                            } else {
                                                "Could not export history."
                                            }
                                    }
                                }
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
                    selectedPeer = runtimeState.value.snapshot.peerDeviceId
                    chooseSource.launch(arrayOf("*/*"))
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
                onClearAudit = {
                    app.auditRecorder.clear { result ->
                        if (!isDestroyed) runOnUiThread {
                            if (!isDestroyed) {
                                auditNotice.value = if (result.isSuccess) {
                                    "Local audit history cleared."
                                } else {
                                    "Audit history could not be cleared."
                                }
                                result.getOrNull()?.let { updateAudit(it) }
                            }
                        }
                    }
                },
                onExportAudit = { exportAudit.launch("crosslab-audit.csv") },
                notificationConsent = notificationConsent.value,
                onToggleNotificationOwner = {
                    app.notificationConsent.setOwnerEnabled(
                        !app.notificationConsent.ownerEnabled,
                    )
                    app.notificationPermissionChanged()
                    refreshNotificationConsent(app)
                },
                onToggleNotificationContent = {
                    app.notificationConsent.setContentEnabled(
                        !app.notificationConsent.contentEnabled,
                    )
                    app.notificationPermissionChanged()
                    refreshNotificationConsent(app)
                },
                onOpenNotificationAccess = {
                    startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
                },
                notificationNotice = notificationNotice.value,
                onSetNotificationPeerPermission = { expectedPeer, allow ->
                    identityWorker.execute {
                        val permitted = runCatching {
                            app.runtimeController.setPermissionForPeer(
                                expectedPeer,
                                "notifications.read",
                                "subscribe",
                                if (allow) RuntimePermissionEffect.ALLOW
                                else RuntimePermissionEffect.DENY,
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
                        if (!isDestroyed) runOnUiThread {
                            if (!isDestroyed) {
                                notificationNotice.value =
                                    if (permitted.getOrDefault(false)) {
                                        "Peer permission saved. A fresh subscription is required."
                                    } else {
                                        "Peer permission could not be changed."
                                    }
                             }
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
                        identityWorker.execute {
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
                            if (!isDestroyed) runOnUiThread {
                                if (!isDestroyed) {
                                    updateTrustedPeerState(identity)
                                    revocationNotice.value =
                                        result.fold(
                                            onSuccess = { "Trust revoked and saved." },
                                            onFailure = { "Revocation could not complete. Check device status and secure storage." },
                                        )
                                }
                            }
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

    private fun updateAudit(history: uniffi.crosslab_mobile_ffi.MobileAuditHistory) {
        auditRows.value = history.rows
        auditDropped.value = history.droppedCount.toLong()
    }

    private fun refreshTrustedPeers(app: CrossLabApplication) {
        identityWorker.execute {
            val identity = runCatching { app.identityRepository.load() }.getOrNull()
            if (!isDestroyed) {
                runOnUiThread {
                    if (!isDestroyed) updateTrustedPeerState(identity)
                }
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
        auditSubscription?.close()
        auditSubscription = null
        runtimeSubscription?.close()
        runtimeSubscription = null
        clipboardSubscription?.close()
        clipboardSubscription = null
        fileTransferSubscription?.close()
        fileTransferSubscription = null
        pairingSubscription?.close()
        pairingSubscription = null
        identityWorker.shutdown()
        super.onDestroy()
    }
}
