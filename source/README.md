# Mechatronics Study Hub — Rust edition

The whole study site rebuilt in Rust.

| Part | What it is |
|---|---|
| `crates/web` | The site itself: every view (Overview, Notes, Flashcards, Quiz, Calculators, PLC Lab, Formula Sheet, Extra), search, notebook, scroll memory and the EN / 中文 / EN+中文 translation layer. Compiled to WebAssembly. |
| `crates/update` | Offline copy + verified updates (SHA-256 per file, atomic swap, path checks). Has tests. |
| `crates/desktop` | Windows app: a native window (WebView2) that serves the built-in site from its own `studyhub://` origin with a strict Content-Security-Policy and checks for updates when online. |
| `content/content.json` | All learning content (topics, Chinese text, flashcards, quiz, formulas, dictionaries, PLC examples). Data only — no code. |
| `web/` | Page shell: the original stylesheet, header markup, and the small loader that starts the WebAssembly module. |
| `tools/` | `build.sh` (web build + manifest), `package-windows.sh`, `export.js` (one-time migration from the old JS site), `diff.js` / `regress.sh` (old-vs-new comparison). |

## Build

```
rustup target add wasm32-unknown-unknown x86_64-pc-windows-gnu
cargo install wasm-bindgen-cli --version 0.2.100
apt install mingw-w64            # for the Windows .exe
tools/build.sh                   # → dist/web  (the website)
tools/package-windows.sh         # → dist/MechatronicsStudyHub-Windows.zip
cargo test -p studyhub_update    # updater tests
```

## Publishing an update

1. Edit `content/content.json` (or the Rust code) and raise `"version"` in `content.json`
   (format `YYYY.MM.DD-n`; it must be higher than the last one).
2. Run `tools/build.sh`.
3. Upload everything in `dist/web/` to the update address (default
   `https://weiqianw.github.io/studyhub/`, e.g. a GitHub repo `studyhub` with Pages turned on).
   Installed apps pick it up the next time they are online.
4. Publish the same `dist/web` files to the website link so both stay in step.

## Checking against the original

`tools/regress.sh` loads the original JavaScript site and this build side by side in Chromium,
drives them through the same clicks (every topic, every tab, cards, quiz, calculators, PLC lab,
formula search, notebook highlight, search box) in all three language modes, and compares the
rendered HTML.
