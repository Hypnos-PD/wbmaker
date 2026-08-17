//! 内嵌静态文件服务器：只监听 127.0.0.1 随机端口，伺服打包进应用的
//! `web/` 前端资源，并提供一个 `/api/save_png` 端点桥接系统保存对话框。
//!
//! 为什么不用 Tauri 的 `tauri://` 自定义协议直接伺服前端：本项目的渲染依赖
//! module worker（`render-worker.js`），自定义协议下 worker 在 macOS/iOS 的
//! WKWebView 上不可靠；localhost HTTP 与网页版行为完全一致。

use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Manager};

/// 保存回调：(文件名, PNG 字节) -> 保存路径；`Err("cancelled")` 表示用户取消。
pub type SaveFn = Box<dyn Fn(&str, &[u8]) -> Result<String, String> + Send + Sync>;

/// 前端资源的根目录。
/// - debug 构建且本机存在仓库 `web/`：直接伺服它（`tauri dev` 即时生效）
/// - 移动端：资源在 APK/App 包内，std::fs 读不到；由原生侧（MainActivity /
///   AppDelegate）在启动时解到应用 data 目录的 `web/`，这里等待解包完成
/// - 桌面 release：读取打包进资源目录的 `web/`
pub fn frontend_root(app: &AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        let dev_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web");
        if dev_root.is_dir() {
            return dev_root;
        }
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    if let Ok(d) = app.path().app_data_dir() {
        let p = d.join("web");
        wait_for_root(&p);
        if p.is_dir() {
            return p;
        }
    }
    if let Ok(d) = app.path().resource_dir() {
        let p = d.join("web");
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::new()
}

/// 移动端资源解包发生在原生侧后台线程；最多等 30s（100ms 轮询）。
#[cfg(any(target_os = "android", target_os = "ios"))]
fn wait_for_root(p: &Path) {
    let marker = p.join("index.html");
    for _ in 0..300 {
        if marker.is_file() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// 启动服务器（保存回调为应用壳的 `commands::save_png`），返回监听端口。
pub fn start(app: AppHandle, root: PathBuf) -> Result<u16, String> {
    let save: SaveFn = Box::new(move |name: &str, bytes: &[u8]| {
        crate::commands::save_png(&app, name, bytes)
    });
    start_with_save(root, save)
}

/// 无 Tauri 依赖的启动入口（单测可用假保存回调）。
pub fn start_with_save(root: PathBuf, save: SaveFn) -> Result<u16, String> {
    // 127.0.0.1:0 → 随机空闲端口，仅本机可达
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|e| format!("bind 127.0.0.1 failed: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .port();
    let server =
        tiny_http::Server::from_listener(listener, None).map_err(|e| format!("server: {e}"))?;
    let root = Arc::new(root);
    let save = Arc::new(save);
    std::thread::Builder::new()
        .name("wbmaker-static".into())
        .spawn(move || {
            for request in server.incoming_requests() {
                handle(&root, &save, request);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(port)
}

fn handle(root: &Path, save: &SaveFn, request: tiny_http::Request) {
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or("/");

    // 导出桥接：POST /api/save_png，body 为 PNG 原始字节，X-Filename 为文件名
    if request.method() == &tiny_http::Method::Post && path == "/api/save_png" {
        handle_save(save, request);
        return;
    }
    if request.method() != &tiny_http::Method::Get {
        respond(request, 405, "text/plain", b"method not allowed");
        return;
    }
    serve_static(root, path, request);
}

fn handle_save(save: &SaveFn, mut request: tiny_http::Request) {
    let mut body = Vec::new();
    if request.as_reader().read_to_end(&mut body).is_err() {
        respond(request, 400, "text/plain", b"bad body");
        return;
    }
    let filename = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("X-Filename"))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_else(|| "card.png".to_string());
    let filename = percent_decode(&filename);
    match save(&filename, &body) {
        Ok(p) => {
            let json = serde_json::json!({ "ok": true, "path": p }).to_string();
            respond(request, 200, "application/json", json.as_bytes());
        }
        Err(e) if e == "cancelled" => {
            respond(request, 200, "application/json", br#"{"ok":false,"cancelled":true}"#);
        }
        Err(e) => {
            let json = serde_json::json!({ "ok": false, "error": e }).to_string();
            respond(request, 500, "application/json", json.as_bytes());
        }
    }
}

fn serve_static(root: &Path, path: &str, request: tiny_http::Request) {
    // 目录请求 → index.html
    let rel = if path == "/" || path.is_empty() {
        "index.html".to_string()
    } else {
        path.trim_start_matches('/').to_string()
    };
    // 路径穿越防护：拒绝 .. 段、拒绝反斜杠（Windows 上同义）
    if rel.split('/').any(|seg| seg == "..") || rel.contains('\\') {
        respond(request, 403, "text/plain", b"forbidden");
        return;
    }
    let full = root.join(&rel);
    match std::fs::read(&full) {
        Ok(mut bytes) => {
            // 独立应用标记：注入到 index.html，前端据此切换「原生保存导出 /
            // 隐藏 WBA 站内链接」等 standalone 行为。
            if rel == "index.html" {
                const MARKER: &[u8] = b"<script>window.__WBMAKER_STANDALONE__=true</script>";
                if let Some(pos) = find_head_end(&bytes) {
                    let mut out = Vec::with_capacity(bytes.len() + MARKER.len());
                    out.extend_from_slice(&bytes[..pos]);
                    out.extend_from_slice(MARKER);
                    out.extend_from_slice(&bytes[pos..]);
                    bytes = out;
                }
            }
            let mime = mime_of(&rel);
            respond(request, 200, mime, &bytes);
        }
        Err(_) => respond(request, 404, "text/plain", b"not found"),
    }
}

/// 找到 `</head>` 的位置（大小写不敏感，HTML 标签大小写无所谓）。
fn find_head_end(bytes: &[u8]) -> Option<usize> {
    let needle = b"</head>";
    let lower: Vec<u8> = bytes.iter().map(|c| c.to_ascii_lowercase()).collect();
    lower
        .windows(needle.len())
        .position(|w| w == needle)
}

/// 极简百分号解码（前端 X-Filename 头经过 encodeURIComponent）。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn mime_of(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "otf" => "font/otf",
        "ttf" => "font/ttf",
        "ttc" => "font/collection",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

fn respond(request: tiny_http::Request, status: u16, mime: &str, bytes: &[u8]) {
    let status_code = tiny_http::StatusCode(status);
    let header = tiny_http::Header::from_bytes("Content-Type", mime)
        .unwrap_or_else(|_| {
            tiny_http::Header::from_bytes("Content-Type", "application/octet-stream").unwrap()
        });
    let response = tiny_http::Response::from_data(bytes.to_vec())
        .with_status_code(status_code)
        .with_header(header);
    let _ = request.respond(response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::Mutex;

    struct Harness {
        root: PathBuf,
        port: u16,
        calls: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    }

    static ROOT_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    impl Harness {
        fn new() -> Self {
            let n = ROOT_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!(
                "wbmaker-srv-test-{}-{n}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("fonts")).unwrap();
            std::fs::write(
                root.join("index.html"),
                b"<html><head><title>x</title></head><body>hi</body></html>",
            )
            .unwrap();
            std::fs::write(root.join("fonts/a.otf"), b"OTTO").unwrap();
            std::fs::write(root.join("app.js"), b"console.log(1)").unwrap();
            let calls: Arc<Mutex<Vec<(String, Vec<u8>)>>> = Arc::new(Mutex::new(Vec::new()));
            let calls2 = calls.clone();
            let save: SaveFn = Box::new(move |name: &str, bytes: &[u8]| {
                if name == "cancel.png" {
                    return Err("cancelled".to_string());
                }
                if name == "boom.png" {
                    return Err("disk full".to_string());
                }
                calls2.lock().unwrap().push((name.to_string(), bytes.to_vec()));
                Ok("/saved/card.png".to_string())
            });
            let port = start_with_save(root.clone(), save).unwrap();
            Harness { root, port, calls }
        }

        fn get(&self, path: &str) -> (u16, Vec<u8>) {
            request(self.port, "GET", path, None, None)
        }
    }

    fn request(port: u16, method: &str, path: &str, body: Option<&[u8]>, filename: Option<&str>) -> (u16, Vec<u8>) {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
        if let Some(f) = filename {
            req.push_str(&format!("X-Filename: {f}\r\n"));
        }
        if let Some(b) = body {
            req.push_str(&format!("Content-Length: {}\r\n", b.len()));
        }
        req.push_str("\r\n");
        stream.write_all(req.as_bytes()).unwrap();
        if let Some(b) = body {
            stream.write_all(b).unwrap();
        }
        let mut resp = Vec::new();
        stream.read_to_end(&mut resp).unwrap();
        let head = String::from_utf8_lossy(&resp);
        let status: u16 = head.split(' ').nth(1).unwrap_or("0").parse().unwrap();
        let body = resp
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|i| resp[i + 4..].to_vec())
            .unwrap_or_default();
        (status, body)
    }

    #[test]
    fn static_serving_and_marker_injection() {
        let h = Harness::new();
        const MARKER: &[u8] = b"window.__WBMAKER_STANDALONE__=true";
        let (status, body) = h.get("/");
        assert_eq!(status, 200);
        assert!(body.windows(MARKER.len()).any(|w| w == MARKER));
        assert!(body.windows(5).any(|w| w == b"<body"));
        let (status, body) = h.get("/app.js");
        assert_eq!(status, 200);
        assert_eq!(body, b"console.log(1)");
        let (status, _) = h.get("/fonts/a.otf");
        assert_eq!(status, 200);
        let (status, _) = h.get("/missing.png");
        assert_eq!(status, 404);
        // 路径穿越
        let (status, _) = h.get("/../../etc/passwd");
        assert_eq!(status, 403);
        let (status, _) = h.get("/..\\windows");
        assert_eq!(status, 403);
    }

    #[test]
    fn save_api_happy_and_errors() {
        let h = Harness::new();
        // 注意：HTTP 头只允许 ASCII，前端会 encodeURIComponent 文件名
        let (status, body) = request(h.port, "POST", "/api/save_png", Some(b"\x89PNG"), Some("card.png"));
        assert_eq!(status, 200);
        assert!(String::from_utf8_lossy(&body).contains("\"ok\":true"));
        assert_eq!(h.calls.lock().unwrap().len(), 1);
        assert_eq!(h.calls.lock().unwrap()[0].0, "card.png");
        assert_eq!(h.calls.lock().unwrap()[0].1, b"\x89PNG");

        // 百分号编码的文件名应被解码
        let (status, _) = request(h.port, "POST", "/api/save_png", Some(b"x"), Some("Masterwork%20Artifact%20%CE%A9.png"));
        assert_eq!(status, 200);
        assert_eq!(h.calls.lock().unwrap()[1].0, "Masterwork Artifact Ω.png");

        let (status, body) = request(h.port, "POST", "/api/save_png", Some(b"x"), Some("cancel.png"));
        assert_eq!(status, 200);
        assert!(String::from_utf8_lossy(&body).contains("\"cancelled\":true"));

        let (status, _) = request(h.port, "POST", "/api/save_png", Some(b"x"), Some("boom.png"));
        assert_eq!(status, 500);

        let (status, _) = request(h.port, "PUT", "/api/save_png", None, None);
        assert_eq!(status, 405);
    }
}
