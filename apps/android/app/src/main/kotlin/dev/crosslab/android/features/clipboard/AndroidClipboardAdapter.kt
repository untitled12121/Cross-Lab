package dev.crosslab.android.features.clipboard

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.os.PersistableBundle

private const val EXTRA_IS_SENSITIVE = "android.content.extra.IS_SENSITIVE"
private const val EXTRA_IS_REMOTE_DEVICE = "android.content.extra.IS_REMOTE_DEVICE"

internal sealed class LocalClipboardRead {
    class Text(val value: String) : LocalClipboardRead()

    data object Unavailable : LocalClipboardRead()

    data object Failed : LocalClipboardRead()
}

internal enum class LocalClipboardWrite {
    SUCCESS,
    FAILED,
}

class AndroidClipboardAdapter(context: Context) {
    private val clipboard =
        context.applicationContext.getSystemService(ClipboardManager::class.java)

    internal fun readText(): LocalClipboardRead =
        try {
            val clip = clipboard.primaryClip ?: return LocalClipboardRead.Unavailable
            val description = clip.description
            if (
                !description.hasMimeType(ClipDescription.MIMETYPE_TEXT_PLAIN) &&
                    !description.hasMimeType(ClipDescription.MIMETYPE_TEXT_HTML)
            ) {
                return LocalClipboardRead.Unavailable
            }
            val text = clip.getItemAt(0).text?.toString()
                ?: return LocalClipboardRead.Unavailable
            LocalClipboardRead.Text(text)
        } catch (_: SecurityException) {
            LocalClipboardRead.Unavailable
        } catch (_: RuntimeException) {
            LocalClipboardRead.Failed
        }

    internal fun writeRemoteText(text: String): LocalClipboardWrite =
        try {
            val clip = ClipData.newPlainText("Cross-Lab clipboard", text)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                clip.description.extras =
                    PersistableBundle().apply {
                        putBoolean(EXTRA_IS_SENSITIVE, true)
                        putBoolean(EXTRA_IS_REMOTE_DEVICE, true)
                    }
            }
            clipboard.setPrimaryClip(clip)
            LocalClipboardWrite.SUCCESS
        } catch (_: RuntimeException) {
            LocalClipboardWrite.FAILED
        }
}
