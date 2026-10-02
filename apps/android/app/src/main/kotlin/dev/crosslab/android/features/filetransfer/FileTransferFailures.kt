package dev.crosslab.android.features.filetransfer

import java.io.IOException
import uniffi.crosslab_mobile_ffi.MobileFileTransferException

internal fun fileTransferFailure(
    error: Throwable,
    fallback: FileTransferFailure,
): FileTransferFailure =
    when (error) {
        is MobileFileTransferException.RemoteDenied -> FileTransferFailure.DENIED
        is MobileFileTransferException.NotConnected -> FileTransferFailure.NOT_CONNECTED
        is MobileFileTransferException.NotNegotiated -> FileTransferFailure.NOT_NEGOTIATED
        is MobileFileTransferException.RemoteUnavailable,
        is MobileFileTransferException.StateUnavailable -> FileTransferFailure.UNAVAILABLE
        is MobileFileTransferException.ResourceLimit -> FileTransferFailure.RESOURCE_LIMIT
        is MobileFileTransferException.Integrity -> FileTransferFailure.INTEGRITY
        is MobileFileTransferException.RetainedState,
        is IOException,
        is SecurityException -> FileTransferFailure.STORAGE
        else -> fallback
    }
