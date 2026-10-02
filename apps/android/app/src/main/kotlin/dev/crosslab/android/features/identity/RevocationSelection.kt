package dev.crosslab.android.features.identity

// Resolve the full credential ID only from the inventory actually confirmed by the owner.
internal fun revocationTarget(
    deviceIds: List<ByteArray>,
    index: Int,
    confirmedGeneration: Int,
    currentGeneration: Int,
): ByteArray? {
    if (confirmedGeneration != currentGeneration) return null
    return deviceIds.getOrNull(index)?.copyOf()
}
