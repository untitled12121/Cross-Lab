package dev.crosslab.android.features.filetransfer

/** An existing stream binding cannot be replaced by a late or duplicate Open. */
internal enum class ReceiveStreamOpenDecision {
    BIND,
    IGNORE,
    REJECT,
}

internal fun receiveStreamOpenDecision(
    streamAlreadyBound: Boolean,
    resumeOffset: ULong,
    expectedOffset: ULong,
): ReceiveStreamOpenDecision =
    when {
        streamAlreadyBound -> ReceiveStreamOpenDecision.IGNORE
        resumeOffset != expectedOffset -> ReceiveStreamOpenDecision.REJECT
        else -> ReceiveStreamOpenDecision.BIND
    }
