package dev.crosslab.android.features.owner

import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/**
 * Owner-controlled disk, policy and identity work cannot block Android's UI thread.
 * Bounded admission is explicit: callers must show a retry result if submit returns false.
 */
internal class OwnerWorkQueue(pendingCapacity: Int = 8) : AutoCloseable {
    private val executor: ThreadPoolExecutor

    init {
        require(pendingCapacity > 0)
        executor = ThreadPoolExecutor(
            1, 1, 0L, TimeUnit.MILLISECONDS,
            ArrayBlockingQueue<Runnable>(pendingCapacity),
            { task -> Thread(task, "crosslab-owner-work").apply { isDaemon = true } },
            ThreadPoolExecutor.AbortPolicy(),
        )
    }

    fun submit(command: () -> Unit): Boolean =
        runCatching {
            executor.execute { runCatching { command() } }
            true
        }.getOrDefault(false)

    /** Accepted owner commands finish in FIFO order; new commands are rejected. */
    override fun close() {
        executor.shutdown()
    }
}
