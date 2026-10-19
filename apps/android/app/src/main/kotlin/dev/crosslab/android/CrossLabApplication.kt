package dev.crosslab.android

import android.app.Application
import android.content.pm.ApplicationInfo
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import dev.crosslab.android.features.audit.AndroidAuditStore
import dev.crosslab.android.features.audit.AndroidAuditRecorder
import dev.crosslab.android.features.clipboard.ClipboardController
import dev.crosslab.android.features.clipboard.UnavailableClipboardPort
import dev.crosslab.android.features.devices.MobileRuntimePort
import dev.crosslab.android.features.devices.ProductPresencePort
import dev.crosslab.android.features.identity.AndroidEd25519Signer
import dev.crosslab.android.features.identity.AndroidIdentityStore
import dev.crosslab.android.features.identity.AndroidProductIdentityRepository
import dev.crosslab.android.features.identity.AndroidSigningSlot
import dev.crosslab.android.features.devices.RuntimeController
import dev.crosslab.android.features.filetransfer.FileTransferController
import dev.crosslab.android.features.filetransfer.UnavailableFileTransferPort
import dev.crosslab.android.features.pairing.PairingJoinerController
import dev.crosslab.android.features.permissions.AndroidPolicyStore
import dev.crosslab.android.features.notifications.NotificationPublisher
import dev.crosslab.android.features.notifications.AndroidNotificationConsent
import dev.crosslab.android.features.networking.LanNetworkTracker

class CrossLabApplication : Application(), DefaultLifecycleObserver {
    lateinit var runtimeController: RuntimeController
        private set

    lateinit var clipboardController: ClipboardController
        private set

    lateinit var fileTransferController: FileTransferController
        private set

    lateinit var identityStore: AndroidIdentityStore
        private set

    lateinit var auditStore: AndroidAuditStore
        private set

    lateinit var auditRecorder: AndroidAuditRecorder
        private set

    private val auditObservations = mutableListOf<AutoCloseable>()

    lateinit var notificationConsent: AndroidNotificationConsent
        private set

    lateinit var localDeviceSigner: AndroidEd25519Signer
        private set

    lateinit var identityRepository: AndroidProductIdentityRepository
        private set

    lateinit var policyStore: AndroidPolicyStore
        private set

    lateinit var pairingController: PairingJoinerController
        private set

    private var notificationPublisher: NotificationPublisher? = null
    private val consentRestart = ThreadPoolExecutor(
        1, 1, 0L, TimeUnit.MILLISECONDS, ArrayBlockingQueue<Runnable>(1),
        { task -> Thread(task, "crosslab-consent-restart").apply { isDaemon = true } },
        ThreadPoolExecutor.DiscardOldestPolicy(),
    )

    fun notificationPublisher(): NotificationPublisher? = notificationPublisher

    fun notificationPermissionChanged() {
        // OS callbacks must still request a trust reload if native subscription teardown fails.
        runCatching { notificationPublisher?.disableNotifications() }
        runCatching {
            consentRestart.execute {
                if (::runtimeController.isInitialized &&
                    runtimeController.state().lifecycle ==
                        dev.crosslab.android.features.devices.RuntimeLifecycle.RUNNING
                ) {
                    runCatching { runtimeController.reloadTrust() }
                }
            }
        }
    }

    private lateinit var connectivityManager: ConnectivityManager
    private val networkLock = Any()
    private val lanNetworks = LanNetworkTracker<Network>()

    private val networkCallback =
        object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                synchronized(networkLock) {
                    if (lanNetworks.appeared(network)) runtimeController.onNetworkAvailable()
                }
            }

            override fun onLost(network: Network) {
                synchronized(networkLock) {
                    if (lanNetworks.disappeared(network)) runtimeController.onNetworkLost()
                }
            }
        }

    override fun onCreate() {
        super<Application>.onCreate()
        identityStore = AndroidIdentityStore(this)
        auditStore = AndroidAuditStore(this)
        auditRecorder = AndroidAuditRecorder(auditStore)
        notificationConsent = AndroidNotificationConsent(this)
        policyStore = AndroidPolicyStore(this)
        localDeviceSigner = AndroidEd25519Signer(this, AndroidSigningSlot.LOCAL_DEVICE)
        identityRepository =
            AndroidProductIdentityRepository(
                store = identityStore,
                rootSigner = AndroidEd25519Signer(this, AndroidSigningSlot.OWNER_ROOT),
                deviceSigningSigner =
                    AndroidEd25519Signer(this, AndroidSigningSlot.DEVICE_SIGNING),
                localDeviceSigner = localDeviceSigner,
            )
        val developmentProvisioning = developmentProvisioningPath()
        val productPresence =
            if (developmentProvisioning == null) {
                ProductPresencePort(
                    this,
                    identityRepository,
                    policyStore,
                    notificationConsent,
                )
            } else {
                null
            }
        notificationPublisher = productPresence
        runtimeController =
            RuntimeController(
                productPresence ?: MobileRuntimePort(checkNotNull(developmentProvisioning)),
            )
        clipboardController =
            ClipboardController(productPresence ?: UnavailableClipboardPort)
        fileTransferController =
            FileTransferController(productPresence ?: UnavailableFileTransferPort)
        pairingController =
            PairingJoinerController(
                context = this,
                identityRepository = identityRepository,
                onPaired = runtimeController::reconnectPeer,
            )

        auditObservations += runtimeController.observe { auditRecorder.runtime(it.snapshot) }
        auditObservations += pairingController.observe { auditRecorder.pairing(it.stage) }
        auditObservations += fileTransferController.observe { auditRecorder.transfer(it) }
        productPresence?.let { presence ->
            auditObservations += presence.observeSubscription(auditRecorder::subscription)
        }
        // Discovery is LAN-only; no Internet/default-network requirement.
        runtimeController.onNetworkLost()
        ProcessLifecycleOwner.get().lifecycle.addObserver(this)

        connectivityManager = getSystemService(ConnectivityManager::class.java)
        val request =
            NetworkRequest.Builder()
                .removeCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .addTransportType(NetworkCapabilities.TRANSPORT_ETHERNET)
                .build()
        connectivityManager.registerNetworkCallback(request, networkCallback)
    }

    private fun developmentProvisioningPath(): String? {
        if (!BuildConfig.CROSSLAB_DEVELOPMENT_PROVISIONING ||
            applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE == 0
        ) {
            return null
        }

        val provisioning = filesDir.resolve("crosslab-development-provisioning.json")
        return provisioning.takeIf { it.isFile }?.absolutePath
    }

    override fun onStart(owner: LifecycleOwner) {
        runtimeController.onForeground()
    }

    override fun onStop(owner: LifecycleOwner) {
        if (pairingController.state().active) {
            runCatching { pairingController.cancel() }
        }
        runtimeController.onBackground()
    }

    override fun onTerminate() {
        runCatching { connectivityManager.unregisterNetworkCallback(networkCallback) }
        runCatching { ProcessLifecycleOwner.get().lifecycle.removeObserver(this) }
        auditObservations.forEach { runCatching { it.close() } }
        auditObservations.clear()
        runCatching { pairingController.close() }
        runCatching { clipboardController.shutdown() }
        runCatching { fileTransferController.shutdown() }
        runCatching { runtimeController.shutdown() }
        runCatching { auditRecorder.close() }
        consentRestart.shutdownNow()
        super.onTerminate()
    }
}
