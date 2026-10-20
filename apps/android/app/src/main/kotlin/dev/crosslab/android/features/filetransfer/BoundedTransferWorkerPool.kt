package dev.crosslab.android.features.filetransfer

import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/**
 * Never use caller-runs for platform I/O: overload must reject promptly rather than
 * execute filesystem/native calls on Android's owner UI thread.
 */
internal class BoundedTransferWorkerPool(
    name: String,
    workers: Int,
    pendingTasks: Int,
) {
    private val executor: ThreadPoolExecutor

    init {
        require(workers > 0 && pendingTasks > 0)
        executor = ThreadPoolExecutor(
            workers,
            workers,
            0L,
            TimeUnit.MILLISECONDS,
            ArrayBlockingQueue<Runnable>(pendingTasks),
            { command -> Thread(command, name).apply { isDaemon = true } },
            ThreadPoolExecutor.AbortPolicy(),
        )
    }

    fun submit(job: () -> Unit): Boolean =
        runCatching {
            executor.execute { runCatching { job() } }
            true
        }.getOrDefault(false)

    fun shutdown() {
        executor.shutdown()
    }

    fun shutdownNow() {
        executor.shutdownNow()
    }
}
