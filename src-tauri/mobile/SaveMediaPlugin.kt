package com.hypd.wbmaker

import android.app.Activity
import android.content.ContentValues
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

// 注意：此文件是模板（src-tauri/mobile/SaveMediaPlugin.kt），由
// tools/package.sh 复制到 gen/android 工程；Rust 侧通过
// register_android_plugin("com.hypd.wbmaker", "SaveMediaPlugin") 加载。
//
// 用途：把导出的 PNG 写入系统相册（MediaStore，Android 10+ 无需权限），
// 返回 content URI——替代写应用私有目录（用户不可见）的做法。
@InvokeArg
class SavePngArgs {
  lateinit var name: String
  lateinit var data: String
}

@TauriPlugin
class SaveMediaPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun savePng(invoke: Invoke) {
    // 注意：入参必须用 @InvokeArg 类解析（JSObject 只能用于构造响应，
    // 用 JSObject 接参数会拿到空值）。
    val args = invoke.parseArgs(SavePngArgs::class.java)
    try {
      val bytes = android.util.Base64.decode(args.data, android.util.Base64.DEFAULT)
      val path = saveToPictures(args.name, bytes)
      invoke.resolve(JSObject().apply { put("ok", true); put("path", path) })
    } catch (e: Exception) {
      android.util.Log.e("wbmakerSave", "savePng failed", e)
      invoke.reject("savePng failed: ${e.message}")
    }
  }

  private fun saveToPictures(name: String, bytes: ByteArray): String {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      val values = ContentValues().apply {
        put(MediaStore.Images.Media.DISPLAY_NAME, name)
        put(MediaStore.Images.Media.MIME_TYPE, "image/png")
        put(MediaStore.Images.Media.RELATIVE_PATH, "Pictures/WB制卡器")
      }
      val resolver = activity.contentResolver
      val uri = resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values)
        ?: throw IllegalStateException("MediaStore insert failed")
      resolver.openOutputStream(uri)?.use { it.write(bytes) }
        ?: throw IllegalStateException("open output stream failed")
      return uri.toString()
    } else {
      @Suppress("DEPRECATION")
      val dir = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_PICTURES)
      val file = File(dir, name)
      file.writeBytes(bytes)
      return file.absolutePath
    }
  }
}
