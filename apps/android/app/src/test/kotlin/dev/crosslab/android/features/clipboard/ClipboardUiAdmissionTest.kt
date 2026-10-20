package dev.crosslab.android.features.clipboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ClipboardUiAdmissionTest {
    @Test
    fun nativeCallbackFailureSchedulesBoundedErrorWithoutEscapingUi() {
        var fallbackCalls = 0
        val queued = ClipboardUiAdmission(1)
        assertTrue(queued.dispatch(post = { task -> task(); true }) {
            invokeClipboardCallback(
                action = { throw IllegalStateException("simulated native clipboard error") },
                onFailure = { fallbackCalls++ },
            )
        })
        assertEquals(1, fallbackCalls)
        assertTrue(queued.dispatch(post = { task -> task(); true }) { })
    }

    @Test
    fun brokenNativeErrorReplyCannotCrashUiOrConsumePermit() {
        var attempted = 0
        val queued = ClipboardUiAdmission(1)
        assertTrue(queued.dispatch(post = { task -> task(); true }) {
            invokeClipboardCallback(
                action = { throw IllegalStateException("fault") },
                onFailure = {
                    attempted++
                    throw IllegalStateException("native error lane unavailable")
                },
            )
        })
        assertEquals(1, attempted)
        assertTrue(queued.dispatch(post = { task -> task(); true }) { })
    }

    @Test
    fun pendingUiCapacityRejectsNewWorkAndRecoversAfterCompletion() {
        val limiter = ClipboardUiAdmission(1)
        var scheduled: (() -> Unit)? = null
        var called = 0

        assertTrue(limiter.dispatch(post = { task -> scheduled = task; true }) { called++ })
        assertFalse(limiter.dispatch(post = { error("must not post when full") }) { called++ })
        assertEquals(0, called)

        checkNotNull(scheduled).invoke()
        assertEquals(1, called)
        assertTrue(limiter.dispatch(post = { task -> task(); true }) { called++ })
        assertEquals(2, called)
    }

    @Test
    fun rejectedAndThrowingPostReleaseCapacity() {
        val limiter = ClipboardUiAdmission(1)
        assertFalse(limiter.dispatch(post = { false }) { error("not queued") })
        assertFalse(limiter.dispatch(post = { error("handler stopped") }) { error("not queued") })
        assertTrue(limiter.dispatch(post = { task -> task(); true }) { })
    }

    @Test
    fun throwingUiCallbackCannotLeakAnAdmissionPermit() {
        val limiter = ClipboardUiAdmission(1)
        var pending: (() -> Unit)? = null
        assertTrue(limiter.dispatch(post = { task -> pending = task; true }) {
            error("simulated dead observer")
        })
        val failed = runCatching { checkNotNull(pending).invoke() }
        assertTrue(failed.isFailure)
        assertTrue(limiter.dispatch(post = { task -> task(); true }) { })
    }

    @Test
    fun synchronousCallbackAndRejectedPostCannotReleasePermitTwice() {
        val limiter = ClipboardUiAdmission(1)
        assertFalse(limiter.dispatch(post = { task -> task(); false }) { })
        var waiting: (() -> Unit)? = null
        assertTrue(limiter.dispatch(post = { task -> waiting = task; true }) { })
        assertFalse(limiter.dispatch(post = { true }) { })
        checkNotNull(waiting).invoke()
        assertTrue(limiter.dispatch(post = { task -> task(); true }) { })
    }
}
