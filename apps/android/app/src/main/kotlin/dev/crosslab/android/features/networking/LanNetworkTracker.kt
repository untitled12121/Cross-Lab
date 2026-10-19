package dev.crosslab.android.features.networking

/** A local network remains available while any matching Wi-Fi or Ethernet route exists. */
internal class LanNetworkTracker<T> {
    private val active = mutableSetOf<T>()

    fun appeared(network: T): Boolean = active.add(network) && active.size == 1

    fun disappeared(network: T): Boolean = active.remove(network) && active.isEmpty()
}
