package dev.crosslab.android.features.networking

internal fun androidNsdServiceType(fullType: String): String? {
    val suffix = "local."
    if (!fullType.endsWith(suffix) || fullType.length <= suffix.length) return null
    return fullType.removeSuffix(suffix)
}
