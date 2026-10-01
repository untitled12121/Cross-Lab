package dev.crosslab.android.features.filetransfer

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import java.io.Closeable
import java.io.File
import java.io.FileInputStream
import java.io.RandomAccessFile
import java.io.InputStream
import java.nio.charset.StandardCharsets

private const val IO_BUFFER_BYTES = 64 * 1024
private const val MAX_DISPLAY_NAME_BYTES = 255
private const val MAX_LOCAL_LOCATOR_BYTES = 8 * 1024

data class AndroidFileTransferSource(
    val uri: Uri,
    val displayName: String,
    val fileSize: Long,
)

class AndroidFileTransferAdapter(context: Context) {
    private val resolver = context.contentResolver
    private val partialDir = context.filesDir.resolve("file-transfer")

    fun inspectSource(uri: Uri): AndroidFileTransferSource? {
        val metadata = queryMetadata(uri)
        val displayName = metadata.first ?: return null
        val fileSize = metadata.second ?: resolver.assetLength(uri) ?: return null
        if (!validDisplayName(displayName) || fileSize < 0) return null

        return AndroidFileTransferSource(
            uri = uri,
            displayName = displayName,
            fileSize = fileSize,
        )
    }

    fun openSource(source: AndroidFileTransferSource): InputStream? =
        resolver.openInputStream(source.uri)

    fun persistSourceGrant(uri: Uri): Boolean =
        persistGrant(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)

    fun persistDestinationGrant(uri: Uri): Boolean =
        persistGrant(uri, Intent.FLAG_GRANT_WRITE_URI_PERMISSION)

    fun encodeLocalLocator(uri: Uri): ByteArray? {
        val bytes = uri.toString().toByteArray(StandardCharsets.UTF_8)
        return bytes.takeIf { it.isNotEmpty() && it.size <= MAX_LOCAL_LOCATOR_BYTES }
    }

    fun decodeLocalLocator(bytes: ByteArray): Uri? {
        if (bytes.isEmpty() || bytes.size > MAX_LOCAL_LOCATOR_BYTES) return null
        val value = bytes.toString(StandardCharsets.UTF_8)
        return runCatching { Uri.parse(value) }.getOrNull()
    }

    fun openPartial(
        transferId: ByteArray,
        durableOffset: Long,
    ): AndroidPartialFile? {
        if (durableOffset < 0) return null
        val file = partialFile(transferId) ?: return null
        val partial = runCatching { RandomAccessFile(file, "rw") }.getOrNull() ?: return null
        return try {
            if (partial.length() < durableOffset) {
                partial.close()
                null
            } else {
                partial.setLength(durableOffset)
                partial.seek(durableOffset)
                AndroidPartialFile(partial)
            }
        } catch (_: Exception) {
            runCatching { partial.close() }
            null
        }
    }

    fun openPartialForVerify(transferId: ByteArray): InputStream? =
        partialFile(transferId)
            ?.takeIf(File::isFile)
            ?.let { runCatching { FileInputStream(it) }.getOrNull() }

    fun partialLength(transferId: ByteArray): Long? =
        partialFile(transferId)
            ?.takeIf(File::isFile)
            ?.length()

    fun publishPartial(
        transferId: ByteArray,
        destination: Uri,
    ): Boolean {
        val partial = partialFile(transferId)?.takeIf(File::isFile) ?: return false
        return runCatching {
            resolver.openOutputStream(destination, "wt")?.use { output ->
                FileInputStream(partial).use { input ->
                    input.copyTo(output, IO_BUFFER_BYTES)
                    output.flush()
                }
            } ?: return@runCatching false
            true
        }.getOrDefault(false)
    }

    fun discardPartial(transferId: ByteArray): Boolean {
        val partial = partialFile(transferId) ?: return false
        return !partial.exists() || partial.delete()
    }

    private fun persistGrant(
        uri: Uri,
        flag: Int,
    ): Boolean =
        runCatching {
            resolver.takePersistableUriPermission(uri, flag)
            true
        }.getOrDefault(false)

    private fun queryMetadata(uri: Uri): Pair<String?, Long?> =
        runCatching {
            resolver.query(
                uri,
                arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE),
                null,
                null,
                null,
            )?.use { cursor ->
                if (!cursor.moveToFirst()) return@use null to null
                val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
                val name =
                    nameIndex
                        .takeIf { it >= 0 && !cursor.isNull(it) }
                        ?.let(cursor::getString)
                val size =
                    sizeIndex
                        .takeIf { it >= 0 && !cursor.isNull(it) }
                        ?.let(cursor::getLong)
                name to size
            } ?: (null to null)
        }.getOrDefault(null to null)

    private fun partialFile(transferId: ByteArray): File? {
        if (transferId.size != 32 || !ensurePartialDir()) return null
        val digits = "0123456789abcdef"
        val name = buildString(64) {
            transferId.forEach { byte ->
                val value = byte.toInt() and 0xff
                append(digits[value ushr 4])
                append(digits[value and 0x0f])
            }
        }
        return partialDir.resolve("$name.partial")
    }

    private fun ensurePartialDir(): Boolean =
        partialDir.isDirectory || (!partialDir.exists() && partialDir.mkdirs())
}

class AndroidPartialFile internal constructor(
    private val file: RandomAccessFile,
) : Closeable {
    fun append(bytes: ByteArray): Boolean =
        runCatching {
            file.write(bytes)
            true
        }.getOrDefault(false)

    fun force(): Boolean =
        runCatching {
            file.fd.sync()
            true
        }.getOrDefault(false)

    fun length(): Long = file.length()

    override fun close() {
        file.close()
    }
}

private fun ContentResolver.assetLength(uri: Uri): Long? =
    runCatching {
        openAssetFileDescriptor(uri, "r")?.use { descriptor ->
            descriptor.length.takeIf { it >= 0 }
        }
    }.getOrNull()

private fun validDisplayName(value: String): Boolean =
    value.isNotEmpty() &&
        value.toByteArray(StandardCharsets.UTF_8).size <= MAX_DISPLAY_NAME_BYTES &&
        value != "." &&
        value != ".." &&
        '/' !in value &&
        '\\' !in value &&
        '\u0000' !in value
