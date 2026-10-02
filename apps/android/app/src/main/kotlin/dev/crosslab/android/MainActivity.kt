package dev.crosslab.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.activity.compose.setContent
import dev.crosslab.android.features.appearance.ThemeId
import dev.crosslab.android.features.clipboard.ClipboardState
import dev.crosslab.android.features.appearance.ThemeParser
import dev.crosslab.android.features.appearance.ThemeResolver
import dev.crosslab.android.features.controlcenter.ControlCenterScreen
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.features.devices.RuntimeSession
import dev.crosslab.android.features.filetransfer.FileTransferState
import dev.crosslab.android.features.pairing.PairingJoinerState

class MainActivity : ComponentActivity() {
    private val runtimeState = mutableStateOf(RuntimeControllerState.initial())
    private val clipboardState = mutableStateOf(ClipboardState.initial(false))
    private val fileTransferState = mutableStateOf(FileTransferState.initial(false))
    private val pairingState = mutableStateOf(PairingJoinerState.idle())
    private var runtimeSubscription: AutoCloseable? = null
    private var clipboardSubscription: AutoCloseable? = null
    private var fileTransferSubscription: AutoCloseable? = null
    private var pairingSubscription: AutoCloseable? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val app = application as CrossLabApplication
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
                onPairingBootstrapScanned = app.pairingController::begin,
                onCancelPairing = app.pairingController::cancel,
            )
        }
    }

    override fun onDestroy() {
        runtimeSubscription?.close()
        runtimeSubscription = null
        clipboardSubscription?.close()
        clipboardSubscription = null
        fileTransferSubscription?.close()
        fileTransferSubscription = null
        pairingSubscription?.close()
        pairingSubscription = null
        super.onDestroy()
    }
}
