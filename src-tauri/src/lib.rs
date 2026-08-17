//! wbmaker 桌面/移动端壳。
//!
//! 前端（`web/`）通过一个只监听 127.0.0.1 随机端口的内嵌静态服务器伺服，
//! WebView 加载 `http://127.0.0.1:<port>/index.html`——而不是 Tauri 的
//! `tauri://` 自定义协议。原因是本项目渲染依赖 module worker
//! （`render-worker.js`），而自定义协议下 worker 在 macOS/iOS 的 WKWebView
//! 上不可靠（见 tauri-apps/tauri#13031）；localhost HTTP 与网页版完全同构，
//! worker / fetch / 字体分块 / Blob 预览全部原样工作。
//!
//! 导出 PNG 同样不走 Tauri IPC（避免自定义协议下的注入限制），而是由前端
//! POST 原始字节到 `/api/save_png`，Rust 侧弹系统保存对话框写盘。

mod commands;
mod server;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    // savemedia：Android 上把导出 PNG 写入系统相册的 Kotlin 插件（无 Rust
    // 命令，仅由 /api/save_png 通过 PluginHandle 调用）。
    #[cfg(target_os = "android")]
    let builder = builder.plugin(commands::savemedia_plugin());
    builder
        .setup(|app| {
            let root = server::frontend_root(app.handle());
            let port = server::start(app.handle().clone(), root)?;
            eprintln!("[wbmaker] frontend served at http://127.0.0.1:{port}/");
            let url: tauri::Url = format!("http://127.0.0.1:{port}/index.html")
                .parse::<tauri::Url>()
                .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            let url = tauri::WebviewUrl::External(url);
            // 窗口在 setup 阶段用目标 URL 直接创建（不依赖 conf 里的占位页 +
            // navigate：navigate 在 Android 上不会把初始加载切换到外部 URL）。
            let window = tauri::WebviewWindowBuilder::new(app, "main", url)
                .title("WB 制卡器 · wbmaker")
                .inner_size(1240.0, 880.0)
                .min_inner_size(760.0, 600.0)
                .build()
                .map_err(|e| e.to_string())?;
            let _ = window.show();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running wbmaker");
}
