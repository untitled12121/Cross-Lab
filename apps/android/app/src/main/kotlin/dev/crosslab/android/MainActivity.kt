package dev.crosslab.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.mutableStateOf
import dev.crosslab.android.features.appearance.ThemeId
import dev.crosslab.android.features.clipboard.ClipboardState
import dev.crosslab.android.features.appearance.ThemeParser
import dev.crosslab.android.features.appearance.ThemeResolver
import dev.crosslab.android.features.controlcenter.ControlCenterScreen
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.features.pairing.PairingJoinerState

class MainActivity : ComponentActivity() {
    private val runtimeState = mutableStateOf(RuntimeControllerState.initial())
    private val clipboardState = mutableStateOf(ClipboardState.initial(false))
    private val pairingState = mutableStateOf(PairingJoinerState.idle())
    private var runtimeSubscription: AutoCloseable? = null
    private var clipboardSubscription: AutoCloseable? = null
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
            ControlCenterScreen(
                theme = theme,
                runtime = runtimeState.value,
                clipboard = clipboardState.value,
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
        pairingSubscription?.close()
        pairingSubscription = null
        super.onDestroy()
    }
}
