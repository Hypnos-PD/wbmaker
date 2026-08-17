#!/usr/bin/env bash
# 打包多端应用（Tauri 2）。
#
# 用法（在对应操作系统上执行）：
#   ./tools/package.sh linux      # Linux：deb / rpm / AppImage
#   ./tools/package.sh win        # Windows：msi / nsis（需在 Windows 上构建）
#   ./tools/package.sh mac        # macOS：app / dmg（需在 macOS 上构建）
#   ./tools/package.sh android    # Android：apk（需 Android SDK/NDK + JDK 17）
#   ./tools/package.sh ios        # iOS：ipa（需 macOS + Xcode + 开发者账号）
#
# 产物位于 src-tauri/target/release/bundle/（android/ios 另有各自产物目录）。
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET="${1:-linux}"
EXTRA_ARGS=("${@:2}")

# 1. 前端构建（wasm + 字体分块 + 背景/纹章资源）
echo "==> build frontend (wasm + fonts)"
bash ./build.sh

# 2. 组装打包用的前端目录：剔除整字体兜底文件（分块字体全部打进包，
#    兜底整文件仅在分块加载失败时使用——本地必不失败，省 ~85MB）。
#    数字字体不是分块，必须保留。web-dist/ 的内容即打包进应用的资源
#    （src-tauri/tauri.conf.json 的 bundle.resources 指向它），已 gitignore。
echo "==> stage web assets (prune whole-font fallbacks)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -r web/. "$STAGE/web/"
rm -f "$STAGE/web/fonts/"*.otf "$STAGE/web/fonts/"*.ttf "$STAGE/web/fonts/"*.ttc
cp assets/fonts/FOT-TsukuAOldMin-Pr6-E.digits.otf "$STAGE/web/fonts/"
rm -f "$STAGE/web/test-node.mjs" "$STAGE/web/bench-node.mjs"
rm -rf web-dist && mkdir -p web-dist
cp -r "$STAGE/web/." web-dist/

# 3. Linux AppImage 工具自举：linuxdeploy 插件脚本若缺失则从 jsdelivr 镜像拉取
#    （tauri-bundler 默认走 raw.githubusercontent.com，部分网络不可达）；
#    并对已知发行版兼容问题打补丁（详见函数注释）。
ensure_linuxdeploy_tools() {
  local dir="${XDG_CACHE_HOME:-$HOME/.cache}/tauri"
  mkdir -p "$dir"
  local mirror="https://cdn.jsdelivr.net/gh"
  [ -s "$dir/linuxdeploy-plugin-gtk.sh" ] || curl -fsSL --retry 3 \
    "$mirror/tauri-apps/linuxdeploy-plugin-gtk@master/linuxdeploy-plugin-gtk.sh" \
    -o "$dir/linuxdeploy-plugin-gtk.sh"
  [ -s "$dir/linuxdeploy-plugin-gstreamer.sh" ] || curl -fsSL --retry 3 \
    "$mirror/tauri-apps/linuxdeploy-plugin-gstreamer@master/linuxdeploy-plugin-gstreamer.sh" \
    -o "$dir/linuxdeploy-plugin-gstreamer.sh"
  chmod +x "$dir"/linuxdeploy-plugin-*.sh
  # Arch 等发行版 pkg-config 给出的 gdk-pixbuf 目录不存在时，插件脚本会因
  # cp/sed 失败而中止；打补丁跳过（png/jpeg 由应用内 Rust 解码，不受影响）。
  if ! grep -q "gdk pixbuf binarydir missing" "$dir/linuxdeploy-plugin-gtk.sh"; then
    sed -i \
      -e 's|copy_tree "$gdk_pixbuf_binarydir" "$APPDIR/"|if [ -d "$gdk_pixbuf_binarydir" ]; then copy_tree "$gdk_pixbuf_binarydir" "$APPDIR/"; else echo "WARNING: gdk pixbuf binarydir missing, skipping"; fi|' \
      -e 's|if \[ -x "$gdk_pixbuf_query" \]; then|if [ -x "$gdk_pixbuf_query" ] \&\& [ -d "$gdk_pixbuf_binarydir" ]; then|' \
      "$dir/linuxdeploy-plugin-gtk.sh"
  fi
  export PATH="$dir:$PATH"
}

# 用 debug keystore 兜底签名（仅当 APK 未签名时）。
# 正式分发请配置正式 keystore（docs/PACKAGING.md），此步会自动跳过。
sign_android_apk() {
  local build_tools="${ANDROID_HOME:-$HOME/android-toolchain/sdk}/build-tools"
  local bt; bt="$(ls -1 "$build_tools" 2>/dev/null | sort -V | tail -1)"
  [ -n "$bt" ] || { echo "WARN: 找不到 build-tools，跳过签名" >&2; return 0; }
  local apksigner="$build_tools/$bt/apksigner" zipalign="$build_tools/$bt/zipalign"
  local unsigned; unsigned="$(find gen/android/app/build/outputs/apk -name '*-unsigned.apk' 2>/dev/null | head -1)"
  [ -n "$unsigned" ] || return 0
  local ks="${HOME}/.android/debug.keystore"
  if [ ! -f "$ks" ]; then
    keytool -genkeypair -keystore "$ks" -alias androiddebugkey \
      -storepass android -keypass android \
      -dname "CN=Android Debug,O=Android,C=US" -keyalg RSA -keysize 2048 \
      -validity 10000
  fi
  local out; out="${unsigned%-unsigned.apk}.apk"
  "$zipalign" -f 4 "$unsigned" /tmp/wbmaker-aligned.apk
  "$apksigner" sign --ks "$ks" --ks-pass pass:android --key-pass pass:android \
    --out "$out" /tmp/wbmaker-aligned.apk
  echo "==> 已用 debug keystore 签名: $out"
}

# 4. Tauri 构建
#    NO_STRIP=1：linuxdeploy 自带旧版 strip 不认新发行版的 .relr.dyn 段，
#    关掉剥离避免 AppImage 打包失败（代价：包体略大）。
echo "==> tauri build ($TARGET)"
cd src-tauri
export PATH="$HOME/.cargo/bin:$PATH"
export NO_STRIP=1
case "$TARGET" in
  linux)
    ensure_linuxdeploy_tools
    cargo tauri build --bundles deb,appimage "${EXTRA_ARGS[@]}"
    ;;
  win)
    cargo tauri build --bundles msi,nsis "${EXTRA_ARGS[@]}"
    ;;
  mac)
    cargo tauri build --bundles app,dmg "${EXTRA_ARGS[@]}"
    ;;
  android)
    # 首次构建需先生成 Android 工程；随后：
    #  1) 应用模板 MainActivity / SaveMediaPlugin（assets 解包 + 相册保存）
    #  2) 用 icons/android 覆盖默认 Tauri 图标（否则桌面图标是 Tauri 默认标）
    #  3) release 明文放行补丁（前端由内嵌服务器伺服在 http://127.0.0.1）
    [ -d gen/android ] || cargo tauri android init
    cp mobile/MainActivity.kt \
      "gen/android/app/src/main/java/com/hypd/wbmaker/MainActivity.kt"
    cp mobile/SaveMediaPlugin.kt \
      "gen/android/app/src/main/java/com/hypd/wbmaker/SaveMediaPlugin.kt"
    cp -r icons/android/. gen/android/app/src/main/res/
    rm -f gen/android/app/src/main/res/drawable/ic_launcher_background.xml \
      gen/android/app/src/main/res/drawable-v24/ic_launcher_foreground.xml
    if ! grep -q 'usesCleartextTraffic"\] = "true"' gen/android/app/build.gradle.kts; then
      sed -i '/getByName("release") {/a\            manifestPlaceholders["usesCleartextTraffic"] = "true"' \
        gen/android/app/build.gradle.kts
    fi
    cargo tauri android build "${EXTRA_ARGS[@]}"
    # 未配置正式 keystore 时产物为 unsigned APK；用 debug keystore 兜底签名，
    # 保证 `package.sh android` 总能产出可安装的 APK（正式分发请配置签名，
    # 见 docs/PACKAGING.md）。
    sign_android_apk
    ;;
  ios)
    cargo tauri ios build "${EXTRA_ARGS[@]}"
    ;;
  *)
    echo "未知目标: $TARGET（linux/win/mac/android/ios）" >&2
    exit 1
    ;;
esac

echo "==> done. 产物见 src-tauri/target/release/bundle/"
