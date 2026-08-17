//! 导出 PNG 的桥接实现：由内嵌服务器 `/api/save_png` 调用。

use tauri::AppHandle;
#[cfg(any(target_os = "android", target_os = "ios"))]
use tauri::Manager;

/// Android 相册插件句柄（savemedia 插件的 Kotlin 实现见
/// src-tauri/mobile/SaveMediaPlugin.kt）。
#[cfg(target_os = "android")]
pub struct SaveMedia(pub tauri::plugin::PluginHandle<tauri::Wry>);

/// 注册 savemedia 插件（仅 Android）：插件构建后在 setup 里把 Kotlin 类
/// 实例挂进 PluginManager，并把句柄放进状态供 save_png 调用。
#[cfg(target_os = "android")]
pub fn savemedia_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri::Manager;
    tauri::plugin::Builder::new("savemedia")
        .setup(|app, api| {
            let handle = api.register_android_plugin("com.hypd.wbmaker", "SaveMediaPlugin")?;
            app.manage(SaveMedia(handle));
            Ok(())
        })
        .build()
}

/// 保存导出的 PNG。
/// 桌面端：系统保存对话框选路径（用户取消返回 `Err("cancelled")`）；
/// 移动端：写入应用 Documents/exports（后续可接系统分享）。
pub fn save_png(app: &AppHandle, filename: &str, bytes: &[u8]) -> Result<String, String> {
    let name = sanitize_filename(filename);

    #[cfg(target_os = "android")]
    {
        // 优先走 Kotlin 插件写入系统相册（MediaStore，用户可见）；失败回退到
        // 应用私有目录（至少保证导出成功，UI 会提示路径）。
        if let Some(state) = app.try_state::<SaveMedia>() {
            #[derive(serde::Deserialize)]
            struct Resp {
                ok: bool,
                #[serde(default)]
                path: String,
                #[serde(default)]
                error: String,
            }
            let resp: Resp = state
                .0
                .run_mobile_plugin(
                    "savePng",
                    serde_json::json!({ "name": name, "data": base64_encode(bytes) }),
                )
                .map_err(|e| e.to_string())?;
            if resp.ok {
                return Ok(resp.path);
            }
            if !resp.error.is_empty() {
                return Err(resp.error);
            }
        }
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("exports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        return Ok(path.display().to_string());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        use std::sync::mpsc;
        use tauri_plugin_dialog::{DialogExt, FilePath};

        let (tx, rx) = mpsc::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let picked = handle
                .dialog()
                .file()
                .set_file_name(&name)
                .add_filter("PNG 图片", &["png"])
                .blocking_save_file();
            let _ = tx.send(picked);
        })
        .map_err(|e| e.to_string())?;

        let picked = rx
            .recv_timeout(std::time::Duration::from_secs(120))
            .map_err(|e| e.to_string())?;
        match picked {
            Some(FilePath::Path(p)) => {
                std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
                Ok(p.display().to_string())
            }
            Some(FilePath::Url(u)) => Err(format!("unexpected path: {u}")),
            None => Err("cancelled".to_string()),
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    let base: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c => c,
        })
        .collect();
    let base = base.trim().trim_end_matches('.');
    if base.is_empty() {
        "card.png".to_string()
    } else if base.to_ascii_lowercase().ends_with(".png") {
        base.to_string()
    } else {
        format!("{base}.png")
    }
}

/// 标准 base64 编码（传给 Kotlin 插件，避开 JSON 里二进制不可传输的问题）。
#[cfg(target_os = "android")]
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}
