package dev.crosslab.android.features.audit

/**
 * One owner-directed audit operation at a time, including the export document picker.
 * UI callbacks must supply the original ticket: stale results never complete newer work.
 */
internal enum class OwnerAuditAction { REFRESH, CLEAR, EXPORT }

internal class OwnerAuditActionGate : AutoCloseable {
    private var nextTicket = 0L
    private var active: Pair<Long, OwnerAuditAction>? = null
    private var closed = false

    @Synchronized
    fun begin(action: OwnerAuditAction): Long? {
        if (closed || active != null) return null
        if (nextTicket == Long.MAX_VALUE) return null
        val ticket = ++nextTicket
        active = ticket to action
        return ticket
    }

    @Synchronized
    fun finish(ticket: Long): Boolean {
        if (closed || active?.first != ticket) return false
        active = null
        return true
    }

    @Synchronized
    fun isBusy(): Boolean = active != null

    @Synchronized
    override fun close() {
        closed = true
        active = null
    }
}
