package dev.crosslab.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.mutableStateOf
import dev.crosslab.android.features.appearance.ThemeId
import dev.crosslab.android.features.appearance.ThemeParser
import dev.crosslab.android.features.appearance.ThemeResolver
import dev.crosslab.android.features.devices.DevicesScreen
import dev.crosslab.android.features.devices.DevicesState
import dev.crosslab.android.features.devices.RuntimeControllerState

class MainActivity : ComponentActivity() {
    private val runtimeState = mutableStateOf(RuntimeControllerState.initial())
    private var runtimeSubscription: AutoCloseable? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val app = application as CrossLabApplication
        runtimeState.value = app.runtimeController.state()
        runtimeSubscription =
            app.runtimeController.observe { state ->
                runOnUiThread { runtimeState.value = state }
            }

        val theme =
            assets.open(ThemeResolver.assetName(ThemeId.AYU_LIGHT))
                .bufferedReader()
                .use { ThemeParser.parse(it.readText()) }

        setContent {
            DevicesScreen(
                theme = theme,
                devices = DevicesState.from(runtimeState.value.snapshot),
                runtime = runtimeState.value,
            )
        }
    }

    override fun onDestroy() {
        runtimeSubscription?.close()
        runtimeSubscription = null
        super.onDestroy()
    }
}
