# Mechatronics Study Hub

Bilingual (English / 中文) study site for ITE Nitec Mechatronics & Robotics, written in Rust and compiled to WebAssembly.

- **Website:** https://weiqianw.github.io/studyhub/
- **Windows app:** works offline and updates itself from this site when online (it reads `manifest.json` and checks every file's SHA-256).
- **Source:** `source/` (build steps in `source/README.md`).

To publish an update: raise `"version"` in `source/content/content.json`, run `source/tools/build.sh`, and copy `source/dist/web/*` to the root of this repo.
