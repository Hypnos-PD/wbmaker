#!/usr/bin/env bash
# Build the Rust core to WebAssembly and generate the JS glue into web/pkg/.
set -euo pipefail
cd "$(dirname "$0")"

echo "==> cargo build (wasm32)"
cargo build --target wasm32-unknown-unknown --release

echo "==> wasm-bindgen"
mkdir -p web/pkg
wasm-bindgen target/wasm32-unknown-unknown/release/wbmaker.wasm \
  --out-dir web/pkg --target web

echo "==> copy fonts"
mkdir -p web/fonts
cp assets/fonts/*.otf assets/fonts/*.ttf assets/fonts/*.ttc web/fonts/ 2>/dev/null || true

if [ ! -f web/font-chunks.json ]; then
  echo "==> font chunks missing, generating (needs fontTools)"
  python3 tools/split_fonts.py || true
fi

echo "==> copy class backgrounds"
mkdir -p web/backgrounds
cp assets/diy/backgrounds/*.jpg web/backgrounds/ 2>/dev/null || true

echo "==> copy crest thumbnails"
mkdir -p web/crests
cp assets/diy/crests/*.png web/crests/ 2>/dev/null || true

echo "==> done. Serve with:  python3 -m http.server -d web 8000"
