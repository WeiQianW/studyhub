#!/bin/bash
# Build the Windows app (cross-compiled with mingw-w64) and zip it with the WebView2 loader.
set -e
cd "$(dirname "$0")/.."
cargo build --release --target x86_64-pc-windows-gnu -p studyhub_desktop
OUT=dist/MechatronicsStudyHub-Windows; rm -rf $OUT && mkdir -p $OUT
cp target/x86_64-pc-windows-gnu/release/MechatronicsStudyHub.exe $OUT/
cp "$(find ~/.cargo/registry/src -path '*webview2-com-sys*/x64/WebView2Loader.dll' | head -1)" $OUT/
cp desktop-README.txt $OUT/README.txt
(cd dist && rm -f MechatronicsStudyHub-Windows.zip && python3 -c "import shutil;shutil.make_archive('MechatronicsStudyHub-Windows','zip','MechatronicsStudyHub-Windows')")
ls -la dist/MechatronicsStudyHub-Windows.zip
