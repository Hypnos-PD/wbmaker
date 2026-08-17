# 多端打包（Tauri 2）

本目录之外的 `src-tauri/` 是应用壳；`tools/package.sh` 负责把 `web/` 前端
（含字体分块、剔除整字体兜底文件）与壳一起打成各平台安装包。

**架构要点**：壳内 Rust 启动一个只监听 `127.0.0.1` 随机端口的静态服务器伺服
前端，WebView 加载 `http://127.0.0.1:<port>/index.html`。不用 Tauri 的
`tauri://` 自定义协议，因为自定义协议下 module worker 在 macOS/iOS WKWebView
上不可靠（[tauri#13031](https://github.com/tauri-apps/tauri/issues/13031)），
而本项目渲染依赖 `render-worker.js`。导出 PNG 由前端 POST 到
`/api/save_png`，Rust 侧弹系统保存对话框（桌面）或写入应用 Documents/exports
（移动端）。

**移动端资源解包**：桌面安装包的资源是真实文件目录，Rust 可直接读；但
Android 的资源打包在 APK 的 `assets/web` 里（`std::fs` 读不到 APK 内文件），
因此 `src-tauri/mobile/MainActivity.kt` 模板会在首次启动时把 `assets/web`
解到应用 data 目录的 `web/`，Rust 侧 `server::frontend_root` 等待解包完成后
伺服该目录（仅首次启动有一次性拷贝开销）。iOS 同理：App 包内资源需在
AppDelegate 里拷贝到应用目录（模板与代码路径已预留，见下文）。

## 通用依赖

- Rust stable + `wasm32-unknown-unknown` target + `wasm-bindgen-cli`（前端构建）
- `cargo install tauri-cli --version "^2" --locked`
- 平台依赖见 [Tauri 官方前置条件](https://v2.tauri.app/start/prerequisites/)：
  - Linux：`webkit2gtk-4.1`、`libappindicator`/`libayatana-appindicator`、`librsvg` 等
  - Windows：WebView2（Win10/11 自带）、MSVC 工具链
  - macOS：Xcode CLT
  - Android：JDK 17 + Android SDK（platform 34+）+ NDK + `aarch64-linux-android`
    等 Rust target（`rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`）
  - iOS：macOS + Xcode + `aarch64-apple-ios` / `aarch64-apple-ios-sim` target

## 打包

```bash
./tools/package.sh linux     # 本机 Linux 构建（deb / AppImage）
./tools/package.sh win       # 需在 Windows 上执行
./tools/package.sh mac       # 需在 macOS 上执行
./tools/package.sh android   # 需 Android SDK/NDK
./tools/package.sh ios       # 需 macOS + Xcode + 开发者账号
```

产物：桌面在 `src-tauri/target/release/bundle/`；Android 在
`src-tauri/gen/android/app/build/outputs/apk/`；iOS 为 Xcode 工程
`src-tauri/gen/apple/`（或用 `tauri ios build` 出 ipa）。

`package.sh` 会先跑 `build.sh`（重建 wasm + 复制字体），再把 `web/` 复制到
`web-dist/` 并剔除 8 个整字体兜底文件（分块字体全部打进包，兜底只在分块
加载失败时触发——本地必不失败；数字字体保留）。`tauri.conf.json` 的
`bundle.resources` 固定指向裁剪后的 `web-dist/`（该目录已 gitignore，
由 `package.sh` 生成）。

> ⚠️ 打包必须走 `package.sh`（它会刷新 `web-dist/`）；直接手跑
> `cargo tauri build` 会打进**过期的**前端资源——设备上表现就是
> 「新功能没生效」。

### 移动端导出行为

- **Android**：导出 PNG 通过 Kotlin 插件（`src-tauri/mobile/SaveMediaPlugin.kt`）
  写入系统相册 `Pictures/WB制卡器/`（Android 10+ 免权限，MediaStore），
  页面显示「已导出 PNG ✓」提示条；插件失败时回退到应用私有目录。
- **iOS**：目前写入应用私有目录（接相册需等价 Swift 实现，见 iOS 专项）。
- 移动端首次启动会把 `assets/web` 解到应用 data 目录；解包版本戳跟随 APK
  的 `lastUpdateTime`，每次（重）安装自动重解，不会残留旧前端文件。

## Android 专项

1. 首次：`export ANDROID_HOME=<sdk路径>`、`export NDK_HOME=<ndk路径>`、
   `export JAVA_HOME=<JDK 17>`，然后 `cd src-tauri && cargo tauri android init`
   （生成 `gen/android` 工程）。
2. **回环明文放行**（必需，否则白屏）：Android 9+ 默认禁明文 HTTP，而本应用
   前端由内嵌服务器伺服在 `http://127.0.0.1:<随机端口>`。`package.sh` 会自动
   在 release 构建类型里打补丁（`manifestPlaceholders["usesCleartextTraffic"]
   = "true"`）；如手动构建，给 `gen/android/app/build.gradle.kts` 的
   `getByName("release")` 块加上这一行（debug 构建已默认放行）。
3. **资源解包模板**：`package.sh` 会把 `src-tauri/mobile/MainActivity.kt`
   覆盖到生成工程的对应路径（首次启动把 `assets/web` 解到 data 目录，供壳内
   服务器伺服）。改模板时同步该文件，勿只改 `gen/`（重新 init 会覆盖）。
3. **签名**：release 需 keystore。生成后配置到 `src-tauri/tauri.conf.json`
   （`bundle.android`）或用 `~/.tauri/<app>.key` + 环境变量
   （`TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PASSWORD`），见
   [Tauri Android 签名文档](https://v2.tauri.app/distribute/sign/android/)；
   未配置时 Tauri 用 debug keystore 签名（可安装测试，不适用于分发）。
4. **验证项（首测）**：立绘上传的 `<input type=file>` 文件选择器。若点击无效，
   在生成的 `MainActivity.kt` 重写 `onShowFileChooser`（WebView 标准回调，
   `startActivityForResult(Intent.ACTION_GET_CONTENT)`）桥接系统文件选择器；
   确认行为后再决定是否固化为壳代码。
5. 移动端导出目前写入应用私有目录 `Documents/exports`（返回值含路径）；
   后续可接系统分享面板（`ACTION_SEND`）或 MediaStore 存入相册。

## iOS 专项

1. `cd src-tauri && cargo tauri ios init`（生成 Xcode 工程）。
2. **资源解包**（与 Android 同因）：在生成的 AppDelegate/入口处把应用包内
   的 `web` 资源拷贝到应用 data 目录的 `web/`（Rust 侧
   `server::frontend_root` 已按 `app_data_dir()/web` 等待并伺服）。参考
   `src-tauri/mobile/MainActivity.kt` 的 Android 实现写一个等价的 Swift
   拷贝（Bundle.main 资源目录 → `Library/Application Support/<id>/web`，
   与 Rust 侧 `app_data_dir` 解析结果一致；具体路径以真机日志为准）。
3. ATS：对 `127.0.0.1` loopback 默认豁免；如遇拦截，在 Info.plist 加
   `NSAllowsLocalNetworking = YES`。
4. 签名：真机/ipa 需要 Apple 开发者账号；无账号可交付 Xcode 工程文档，
   或先用模拟器调试。
5. 已知现象：Tauri 会向 http 页面注入 IPC 初始化脚本，在部分 WebView 上
   抛 `Cannot redefine property: postMessage` 之类的无害异常（本应用不走
   Tauri IPC，不受影响，可忽略）。

## 调试

```bash
cd src-tauri
cargo tauri dev      # debug 壳 + 直接伺服 ../web（前端改动即时生效）
```

独立应用内前端会检测 `window.__WBMAKER_STANDALONE__`（由壳服务器注入）：
- 导出 PNG → 走 `/api/save_png`（系统保存对话框）
- 隐藏 WBA 站内导航（logo / 返回 WBA），保留样式切换与语言切换

## 已知约束

- Windows 未签名安装包会有 SmartScreen 提示；macOS 未公证需右键打开。
  本地分发可接受；上架/公证另需证书（见 `tauri signer` 流程）。
- 包体 ~85MB/端（字体分块 74MB 占大头）；后续可只打单语言分块。
- iOS 的 ipa 只能在 macOS 上构建。
