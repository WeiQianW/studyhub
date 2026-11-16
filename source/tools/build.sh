#!/bin/bash
# Build the Rust/WASM site into dist/web (index.html, pkg/, content.json, manifest.json)
set -e
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown -p studyhub_web
rm -rf dist/web && mkdir -p dist/web/pkg
wasm-bindgen --target web --no-typescript --out-dir dist/web/pkg target/wasm32-unknown-unknown/release/studyhub_web.wasm
{ cat web/head.html; echo '<style>'; cat web/style.css; echo '</style>'; cat web/body.html; cat web/boot.html; } > dist/web/index.html
cp content/content.json dist/web/content.json
python3 tools/manifest.py dist/web
ls -la dist/web dist/web/pkg
