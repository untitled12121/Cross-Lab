package dev.crosslab.android.features.pairing

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.google.mlkit.vision.barcode.BarcodeScanner
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.common.InputImage
import dev.crosslab.android.components.ui.ControlButton
import dev.crosslab.android.features.appearance.ThemeDocument
import dev.crosslab.android.features.appearance.toComposeColor
import dev.crosslab.android.features.appearance.toTextStyle
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import uniffi.crosslab_mobile_ffi.MobilePairingBootstrap
import uniffi.crosslab_mobile_ffi.scanPairingBootstrap

@Composable
fun PairingScannerPanel(
    theme: ThemeDocument,
    onScanned: (MobilePairingBootstrap) -> Unit,
    onCancel: () -> Unit,
) {
    val context = LocalContext.current
    var permissionGranted by
        remember {
            mutableStateOf(
                context.checkSelfPermission(Manifest.permission.CAMERA) ==
                    PackageManager.PERMISSION_GRANTED,
            )
        }
    val permissionLauncher =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            permissionGranted = granted
        }

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .border(
                    BorderStroke(
                        theme.metrics.borderWidth.toFloat().dp,
                        theme.colors.border.toComposeColor(),
                    ),
                    RectangleShape,
                )
                .padding(theme.spacing.lg.toFloat().dp),
    ) {
        BasicText(
            text = "Scan Add Device QR",
            style =
                theme.typography.scales.label
                    .toTextStyle()
                    .copy(color = theme.colors.foreground.toComposeColor()),
        )
        Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
        BasicText(
            text =
                "Camera frames stay on this device. Cross-Lab accepts only its versioned pairing QR and keeps the one-time secret inside Rust.",
            style =
                theme.typography.scales.caption
                    .toTextStyle()
                    .copy(color = theme.colors.mutedForeground.toComposeColor()),
        )
        Spacer(Modifier.height(theme.spacing.lg.toFloat().dp))

        if (permissionGranted) {
            PairingCameraPreview(
                theme = theme,
                onScanned = onScanned,
            )
        } else {
            BasicText(
                text = "Camera permission is required only while scanning a pairing QR.",
                style =
                    theme.typography.scales.body
                        .toTextStyle()
                        .copy(color = theme.colors.mutedForeground.toComposeColor()),
            )
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
            ControlButton(
                theme = theme,
                label = "Allow camera",
                onClick = { permissionLauncher.launch(Manifest.permission.CAMERA) },
            )
        }

        Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
        ControlButton(
            theme = theme,
            label = "Cancel",
            onClick = onCancel,
        )
    }
}

@Composable
private fun PairingCameraPreview(
    theme: ThemeDocument,
    onScanned: (MobilePairingBootstrap) -> Unit,
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val currentOnScanned by rememberUpdatedState(onScanned)
    var cameraError by remember { mutableStateOf<String?>(null) }
    val previewView =
        remember {
            PreviewView(context).apply {
                scaleType = PreviewView.ScaleType.FILL_CENTER
                implementationMode = PreviewView.ImplementationMode.PERFORMANCE
            }
        }

    DisposableEffect(lifecycleOwner, previewView) {
        val cameraExecutor =
            Executors.newSingleThreadExecutor { task ->
                Thread(task, "crosslab-pairing-camera").apply {
                    isDaemon = true
                }
            }
        val scanner =
            BarcodeScanning.getClient(
                BarcodeScannerOptions.Builder()
                    .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
                    .build(),
            )
        val providerFuture = ProcessCameraProvider.getInstance(context)
        val analyzer =
            PairingQrAnalyzer(scanner) { bootstrap ->
                previewView.post {
                    currentOnScanned(bootstrap)
                }
            }
        val mainExecutor = java.util.concurrent.Executor { task -> previewView.post(task) }

        providerFuture.addListener(
            {
                runCatching {
                    val provider = providerFuture.get()
                    val preview =
                        Preview.Builder()
                            .build()
                            .also { it.surfaceProvider = previewView.surfaceProvider }
                    val analysis =
                        ImageAnalysis.Builder()
                            .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                            .build()
                            .also { it.setAnalyzer(cameraExecutor, analyzer) }

                    provider.unbindAll()
                    provider.bindToLifecycle(
                        lifecycleOwner,
                        CameraSelector.DEFAULT_BACK_CAMERA,
                        preview,
                        analysis,
                    )
                }.onFailure {
                    cameraError = "Camera is unavailable for pairing."
                }
            },
            mainExecutor,
        )

        onDispose {
            runCatching {
                if (providerFuture.isDone) {
                    providerFuture.get().unbindAll()
                }
            }
            scanner.close()
            cameraExecutor.shutdownNow()
        }
    }

    AndroidView(
        factory = { previewView },
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = 280.dp, max = 420.dp)
                .border(
                    BorderStroke(
                        theme.metrics.borderWidth.toFloat().dp,
                        theme.colors.border.toComposeColor(),
                    ),
                    RectangleShape,
                ),
    )

    cameraError?.let { message ->
        Spacer(Modifier.height(theme.spacing.sm.toFloat().dp))
        BasicText(
            text = message,
            style =
                theme.typography.scales.caption
                    .toTextStyle()
                    .copy(color = theme.colors.destructive.toComposeColor()),
        )
    }
}

private class PairingQrAnalyzer(
    private val scanner: BarcodeScanner,
    private val onScanned: (MobilePairingBootstrap) -> Unit,
) : ImageAnalysis.Analyzer {
    private val processing = AtomicBoolean(false)
    private val accepted = AtomicBoolean(false)

    @androidx.annotation.OptIn(ExperimentalGetImage::class)
    override fun analyze(imageProxy: ImageProxy) {
        if (accepted.get() || !processing.compareAndSet(false, true)) {
            imageProxy.close()
            return
        }

        val image = imageProxy.image
        if (image == null) {
            processing.set(false)
            imageProxy.close()
            return
        }

        val input =
            InputImage.fromMediaImage(
                image,
                imageProxy.imageInfo.rotationDegrees,
            )

        scanner.process(input)
            .addOnSuccessListener { barcodes ->
                if (accepted.get()) return@addOnSuccessListener

                for (barcode in barcodes) {
                    val rawValue = barcode.rawValue ?: continue
                    val bootstrap =
                        runCatching { scanPairingBootstrap(rawValue) }
                            .getOrNull()
                            ?: continue

                    if (accepted.compareAndSet(false, true)) {
                        onScanned(bootstrap)
                    }
                    break
                }
            }
            .addOnCompleteListener {
                processing.set(false)
                imageProxy.close()
            }
    }
}
