package com.hypd.wbmaker

import android.content.Context
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import java.io.File
import java.io.FileOutputStream

// 注意：此文件是模板（src-tauri/mobile/MainActivity.kt），由 tools/package.sh
// 在 `tauri android init` 之后覆盖到 gen/android 对应路径。
class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    // 前端资源打包在 APK 的 assets/web 里（Rust 侧 std::fs 读不到 APK 内文件），
    // 后台线程解到 dataDir/web，由壳内 Rust 静态服务器伺服（Rust 侧会等待）。
    Thread { extractWebAssets() }.start()
    super.onCreate(savedInstanceState)
  }

  private fun extractWebAssets() {
    val dest = File(dataDir, "web")
    // 以 APK 的 lastUpdateTime 作为资源版本戳：每次（重）安装后重新解包，
    // 避免设备上残留旧版本的前端文件（早期版本缺此标记，会全量重解一次）。
    val stamp = try {
      packageManager.getPackageInfo(packageName, 0).lastUpdateTime.toString()
    } catch (e: Exception) { "0" }
    val marker = File(dest, ".wbmaker-version")
    val current = try { marker.readText().trim() } catch (e: Exception) { "" }
    if (current == stamp && File(dest, "index.html").exists()) return
    try {
      dest.deleteRecursively()
      copyAssets(this, "web", dest)
      marker.writeText(stamp)
    } catch (e: Exception) {
      android.util.Log.e("wbmaker", "extract web assets failed", e)
    }
  }

  private fun copyAssets(context: Context, src: String, dest: File) {
    val am = context.assets
    fun walk(rel: String) {
      val children = am.list(rel) ?: return
      if (children.isEmpty()) {
        // 是文件：assets 里的相对路径（去掉 "web/" 前缀）
        val out = File(dest, rel.removePrefix("web/"))
        out.parentFile?.mkdirs()
        am.open(rel).use { input ->
          FileOutputStream(out).use { output -> input.copyTo(output) }
        }
      } else {
        children.forEach { walk("$rel/$it") }
      }
    }
    walk(src)
  }
}
