#!/bin/bash
# One command to publish everything to the GitHub Pages repo:
#   website + update manifest (installed apps update from these),
#   a freshly built Windows app containing the same latest notes, the download page, and the source.
# Usage: tools/publish.sh /path/to/studyhub-repo   (then commit and push the repo)
set -e
cd "$(dirname "$0")/.."
REPO="$1"; [ -d "$REPO/.git" ] || { echo "usage: $0 REPO_DIR"; exit 1; }
tools/build.sh >/dev/null
tools/package-windows.sh >/dev/null
rm -rf "$REPO/pkg" && cp -r dist/web/. "$REPO/"
touch "$REPO/.nojekyll"
mkdir -p "$REPO/download"
cp dist/MechatronicsStudyHub-Windows.zip "$REPO/download/"
CONTENT=$(python3 -c "import json;print(json.load(open('content/content.json',encoding='utf-8'))['version'])")
DESKTOP=$(grep -m1 '^version' crates/desktop/Cargo.toml | cut -d'"' -f2)
SIZE=$(python3 -c "import os;print(round(os.path.getsize('dist/MechatronicsStudyHub-Windows.zip')/1e6,1))")
sed -e "s/{{CONTENT}}/$CONTENT/" -e "s/{{DESKTOP}}/$DESKTOP/" -e "s/{{SIZE}}/$SIZE/" web/download.html > "$REPO/download/index.html"
rm -rf "$REPO/source" && mkdir -p "$REPO/source"
tar --exclude=./target --exclude=./dist -cf - . | tar -xf - -C "$REPO/source"
echo "published notes $CONTENT, app $DESKTOP ($SIZE MB) to $REPO"
