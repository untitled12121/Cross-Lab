package dev.crosslab.android.features.pairing

import android.content.Context
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicLong
import dev.crosslab.android.features.identity.AndroidProductIdentityRepository
import uniffi.crosslab_mobile_ffi.MobilePairingBootstrap
import uniffi.crosslab_mobile_ffi.MobileProductPairingJoinerSession
import uniffi.crosslab_mobile_ffi.startProductPairingJoiner

enum class PairingJoinerStage {
    IDLE,
    FINDING_DEVICE,
    CONNECTING,
    SAVING_TRUST,
    FINALIZING,
    PAIRED,
    FAILED,
    CANCELLED,
}

data class PairingJoinerState(
    val stage: PairingJoinerStage,
    val ownerId: String? = null,
    val inviterDeviceId: String? = null,
    val message: String? = null,
) {
    val active: Boolean
        get() =
            stage == PairingJoinerStage.FINDING_DEVICE ||
                stage == PairingJoinerStage.CONNECTING ||
                stage == PairingJoinerStage.SAVING_TRUST ||
                stage == PairingJoinerStage.FINALIZING

    companion object {
        fun idle(): PairingJoinerState = PairingJoinerState(PairingJoinerStage.IDLE)
    }
}

class PairingJoinerController(
    context: Context,
    private val identityRepository: AndroidProductIdentityRepository,
) : AutoCloseable {
    private val discovery = AndroidPairingDiscovery(context)
    private val executor =
        Executors.newSingleThreadExecutor { task ->
            Thread(task, "crosslab-product-pairing").apply { isDaemon = true }
        }
    private val listeners = CopyOnWriteArraySet<(PairingJoinerState) -> Unit>()
    private val generation = AtomicLong(0)

    @Volatile
    private var current = PairingJoinerState.idle()

    @Volatile
    private var discoveryHandle: AutoCloseable? = null

    @Volatile
    private var session: MobileProductPairingJoinerSession? = null

    fun state(): PairingJoinerState = current

    fun observe(listener: (PairingJoinerState) -> Unit): AutoCloseable {
        listeners += listener
        listener(current)
        return AutoCloseable { listeners -= listener }
    }

    fun begin(bootstrap: MobilePairingBootstrap) {
        cancelActive(notify = false)

        val summary =
            runCatching { bootstrap.summary() }
                .getOrElse {
                    publish(
                        PairingJoinerState(
                            stage = PairingJoinerStage.FAILED,
                            message = "The scanned invitation is no longer available.",
                        ),
                    )
                    return
                }

        val token = generation.incrementAndGet()
        publish(
            PairingJoinerState(
                stage = PairingJoinerStage.FINDING_DEVICE,
                ownerId = summary.ownerId,
                inviterDeviceId = summary.inviterDeviceId,
                message = "Finding the inviting device on this local network…",
            ),
        )

        val handle =
            discovery.start(
                bootstrap = bootstrap,
                onResolved = { route ->
                    if (!isCurrent(token)) return@start
                    discoveryHandle = null
                    connect(token, bootstrap, route)
                },
                onFailure = { failure ->
                    if (!isCurrent(token)) return@start
                    discoveryHandle = null
                    fail(
                        token,
                        when (failure) {
                            PairingDiscoveryFailure.INVALID_INVITATION ->
                                "The pairing invitation is invalid or expired."
                            PairingDiscoveryFailure.START_FAILED ->
                                "Local device discovery could not start."
                            PairingDiscoveryFailure.RESOLVE_FAILED ->
                                "The inviting device could not be resolved."
                            PairingDiscoveryFailure.PROFILE_MISMATCH ->
                                "The discovered service does not match the Cross-Lab pairing profile."
                            PairingDiscoveryFailure.TIMEOUT ->
                                "The inviting device was not found before the discovery timeout."
                        },
                    )
                },
            )

        if (isCurrent(token)) {
            discoveryHandle = handle
        } else {
            handle.close()
        }
    }

    fun cancel() {
        if (current.active) {
            cancelActive(notify = true)
        } else {
            generation.incrementAndGet()
            publish(PairingJoinerState.idle())
        }
    }

    private fun connect(
        token: Long,
        bootstrap: MobilePairingBootstrap,
        route: PairingResolvedRoute,
    ) {
        publishIfCurrent(
            token,
            current.copy(
                stage = PairingJoinerStage.CONNECTING,
                message = "Connecting and verifying the one-time pairing transcript…",
            ),
        )

        executor.execute {
            val pairingSession =
                runCatching {
                    startProductPairingJoiner(
                        bootstrap = bootstrap,
                        address = route.addressBytes,
                        port = route.port,
                        scopeId = route.scopeId,
                        localDeviceSigner = identityRepository.localDeviceSigner,
                    )
                }.getOrElse {
                    fail(token, "Secure pairing with the inviting device failed.")
                    return@execute
                }

            if (!isCurrent(token)) {
                runCatching { pairingSession.cancel() }
                return@execute
            }
            session = pairingSession

            publishIfCurrent(
                token,
                current.copy(
                    stage = PairingJoinerStage.SAVING_TRUST,
                    message = "Pairing verified. Saving device trust securely…",
                ),
            )

            val persisted =
                runCatching {
                    identityRepository.commitJoinerPairing(pairingSession.completion())
                }.isSuccess

            if (!persisted) {
                runCatching { pairingSession.persistenceFailed() }
                session = null
                fail(token, "Pairing was verified, but secure trust storage failed.")
                return@execute
            }

            publishIfCurrent(
                token,
                current.copy(
                    stage = PairingJoinerStage.FINALIZING,
                    message = "Trust saved. Waiting for the inviting device to confirm completion…",
                ),
            )

            val finished = runCatching { pairingSession.finishPersisted() }.isSuccess
            session = null
            if (!isCurrent(token)) return@execute

            if (!finished) {
                fail(
                    token,
                    "Trust was saved, but final peer confirmation was interrupted. " +
                        "Keep both devices on the same network and reconnect.",
                )
                return@execute
            }

            publish(
                current.copy(
                    stage = PairingJoinerStage.PAIRED,
                    message = "Device paired successfully.",
                ),
            )
        }
    }

    private fun cancelActive(notify: Boolean) {
        generation.incrementAndGet()
        discoveryHandle?.close()
        discoveryHandle = null

        val activeSession = session
        session = null
        if (activeSession != null) {
            executor.execute { runCatching { activeSession.cancel() } }
        }

        if (notify) {
            publish(
                current.copy(
                    stage = PairingJoinerStage.CANCELLED,
                    message = "Pairing cancelled.",
                ),
            )
        }
    }

    private fun fail(
        token: Long,
        message: String,
    ) {
        if (!isCurrent(token)) return
        publish(
            current.copy(
                stage = PairingJoinerStage.FAILED,
                message = message,
            ),
        )
    }

    private fun publishIfCurrent(
        token: Long,
        state: PairingJoinerState,
    ) {
        if (isCurrent(token)) publish(state)
    }

    private fun isCurrent(token: Long): Boolean = generation.get() == token

    private fun publish(state: PairingJoinerState) {
        current = state
        listeners.forEach { it(state) }
    }

    override fun close() {
        cancelActive(notify = false)
        listeners.clear()
        executor.shutdownNow()
    }
}
