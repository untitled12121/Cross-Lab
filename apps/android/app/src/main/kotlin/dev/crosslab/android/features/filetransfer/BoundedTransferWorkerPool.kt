package dev.crosslab.android.features.filetransfer

// File-transfer tests and service retain their scoped name; execution is shared.
internal typealias BoundedTransferWorkerPool =
    dev.crosslab.android.features.workers.BoundedWorkerPool
