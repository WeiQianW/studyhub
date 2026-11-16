# Mechatronics Study Hub

Bilingual (English / 中文) study site for ITE Nitec Mechatronics & Robotics, written in Rust and compiled to WebAssembly.

- **Website:** https://weiqianw.github.io/studyhub/
- **Download the Windows app:** https://weiqianw.github.io/studyhub/download/ (or [the zip directly](download/MechatronicsStudyHub-Windows.zip))
- **How the app stays current:** works offline and updates itself from this site when online (it reads `manifest.json` and checks every file's SHA-256).
- **Source:** `source/` (build steps in `source/README.md`).

To publish an update: raise `"version"` in `source/content/content.json` (and the version in `source/crates/desktop/Cargo.toml` if the app program itself changed), then run `source/tools/publish.sh <this repo>` and push. That rebuilds the website, the update manifest and the downloadable app with the same latest notes.
