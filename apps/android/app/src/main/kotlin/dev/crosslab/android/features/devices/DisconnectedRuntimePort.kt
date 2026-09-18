package dev.crosslab.android.features.devices

class DisconnectedRuntimePort : RuntimePort {
    override fun start() = Unit

    override fun stop() = Unit

    override fun networkLost() = Unit

    override fun networkAvailable() = Unit
}
