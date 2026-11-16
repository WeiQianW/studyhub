//! Offline copy and verified updates for the desktop app.
//! Layout under the data folder: `current/` (installed update), `next/` (being downloaded), `old/` (during swap).
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MAX_FILE: u64 = 64 * 1024 * 1024;

#[derive(Deserialize, Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Deserialize, Clone, Debug)]
pub struct Manifest {
    pub app: String,
    pub version: String,
    pub files: Vec<FileEntry>,
    /// Latest version of the Windows app itself (the .exe), for the "new app available" notice.
    #[serde(default)]
    pub desktop: Option<String>,
    /// The app's own program files for that version (self-update), e.g. `download/app/MechatronicsStudyHub.exe`.
    #[serde(default)]
    pub desktop_files: Vec<FileEntry>,
}

/// Program files the app may replace in its own folder.
pub const APP_FILES: [&str; 2] = ["MechatronicsStudyHub.exe", "WebView2Loader.dll"];

pub type Bundle = &'static [(&'static str, &'static [u8])];

pub struct Store {
    pub dir: PathBuf,
    pub bundle: Bundle,
}

/// "2026.11.16-1" → [2026, 11, 16, 1]
pub fn ver_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse().unwrap_or(0)).collect()
}
pub fn sha_hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{:02x}", x)).collect()
}
/// Only plain relative paths like `pkg/a.js` — no absolute paths, drive letters, `..` or backslashes.
pub fn safe_rel(p: &str) -> bool {
    !p.is_empty() && !p.starts_with('/') && !p.contains('\\') && !p.contains(':') && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}
pub fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        _ => "application/octet-stream",
    }
}

impl Store {
    pub fn current(&self) -> PathBuf {
        self.dir.join("current")
    }
    pub fn bundled(&self) -> Manifest {
        let raw = self.bundle.iter().find(|f| f.0 == "manifest.json").expect("bundled manifest").1;
        serde_json::from_slice(raw).expect("bundled manifest json")
    }
    /// The downloaded copy, when it is complete, newer than the built-in one, and every file is present.
    pub fn cached(&self) -> Option<Manifest> {
        let dir = self.current();
        let m: Manifest = serde_json::from_slice(&fs::read(dir.join("manifest.json")).ok()?).ok()?;
        if m.app != "studyhub" || ver_key(&m.version) <= ver_key(&self.bundled().version) {
            return None;
        }
        for f in &m.files {
            if !safe_rel(&f.path) || fs::metadata(dir.join(&f.path)).ok()?.len() != f.size {
                return None;
            }
        }
        Some(m)
    }
    pub fn active_version(&self) -> String {
        self.cached().map(|m| m.version).unwrap_or_else(|| self.bundled().version)
    }
    /// Bytes for one request path. Files from the downloaded copy are re-hashed on every read,
    /// so a damaged or altered file is never served.
    pub fn serve(&self, path: &str) -> Option<Vec<u8>> {
        let path = if path.is_empty() { "index.html" } else { path };
        if let Some(m) = self.cached() {
            let e = m.files.iter().find(|f| f.path == path)?;
            let b = fs::read(self.current().join(&e.path)).ok()?;
            return if sha_hex(&b) == e.sha256 { Some(b) } else { None };
        }
        self.bundle.iter().find(|f| f.0 == path).map(|f| f.1.to_vec())
    }

    /// Check `base` (ending in '/') for a newer version and install it. Returns the new version.
    pub fn check(&self, base: &str, allow_http: bool) -> Result<Option<String>, String> {
        self.check_full(base, allow_http).map(|r| r.0)
    }
    /// Like `check`, and also returns the newest desktop-app version named in the manifest.
    pub fn check_full(&self, base: &str, allow_http: bool) -> Result<(Option<String>, Option<String>), String> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(60))).https_only(!allow_http).build().into();
        let raw = get(&agent, &format!("{}manifest.json", base))?;
        let m: Manifest = serde_json::from_slice(&raw).map_err(|e| format!("manifest: {}", e))?;
        let desk = m.desktop.clone();
        if m.app != "studyhub" || ver_key(&m.version) <= ver_key(&self.active_version()) {
            return Ok((None, desk));
        }
        if m.files.iter().all(|f| f.path != "index.html") || m.files.iter().any(|f| !safe_rel(&f.path) || f.size > MAX_FILE || f.path == "manifest.json") {
            return Err("manifest rejected".into());
        }
        let stage = self.dir.join("next");
        let _ = fs::remove_dir_all(&stage);
        fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
        for f in &m.files {
            let have = fs::read(self.current().join(&f.path))
                .ok()
                .filter(|b| sha_hex(b) == f.sha256)
                .or_else(|| self.bundle.iter().find(|x| x.0 == f.path && sha_hex(x.1) == f.sha256).map(|x| x.1.to_vec()));
            let bytes = match have {
                Some(b) => b,
                None => {
                    let b = get(&agent, &format!("{}{}", base, f.path))?;
                    if b.len() as u64 != f.size || sha_hex(&b) != f.sha256 {
                        return Err(format!("{}: checksum mismatch", f.path));
                    }
                    b
                }
            };
            write(&stage.join(&f.path), &bytes)?;
        }
        write(&stage.join("manifest.json"), &raw)?;
        let cur = self.current();
        let old = self.dir.join("old");
        let _ = fs::remove_dir_all(&old);
        if cur.exists() {
            fs::rename(&cur, &old).map_err(|e| e.to_string())?;
        }
        if let Err(e) = fs::rename(&stage, &cur) {
            let _ = fs::rename(&old, &cur);
            return Err(e.to_string());
        }
        let _ = fs::remove_dir_all(&old);
        Ok((Some(m.version), desk))
    }
}

impl Store {
    /// Download the newest app program files into `app-next/`, each verified against the manifest.
    /// Returns (version, [(file name, staged path)]).
    pub fn fetch_app(&self, base: &str, allow_http: bool) -> Result<(String, Vec<(String, PathBuf)>), String> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(300))).https_only(!allow_http).build().into();
        let m: Manifest = serde_json::from_slice(&get(&agent, &format!("{}manifest.json", base))?).map_err(|e| format!("manifest: {}", e))?;
        let ver = m.desktop.clone().ok_or("no app version published")?;
        if m.desktop_files.is_empty() {
            return Err("no app files published".into());
        }
        let stage = self.dir.join("app-next");
        let _ = fs::remove_dir_all(&stage);
        fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
        let mut out = vec![];
        for f in &m.desktop_files {
            let name = f.path.rsplit('/').next().unwrap_or("").to_string();
            if !safe_rel(&f.path) || !APP_FILES.contains(&name.as_str()) || f.size > MAX_FILE {
                return Err(format!("{}: not an app file", f.path));
            }
            let b = get(&agent, &format!("{}{}", base, f.path))?;
            if b.len() as u64 != f.size || sha_hex(&b) != f.sha256 {
                return Err(format!("{}: checksum mismatch", f.path));
            }
            let p = stage.join(&name);
            write(&p, &b)?;
            out.push((name, p));
        }
        Ok((ver, out))
    }
}

/// Put staged program files in place next to the running app. Windows lets a running .exe/.dll be
/// renamed (not overwritten), so each old file is renamed to `*.old` first; on any failure the
/// originals are restored.
pub fn install_app(dir: &Path, files: &[(String, PathBuf)]) -> Result<(), String> {
    let mut done: Vec<(PathBuf, PathBuf)> = vec![];
    let undo = |done: &Vec<(PathBuf, PathBuf)>| {
        for (cur, old) in done.iter().rev() {
            let _ = fs::remove_file(cur);
            let _ = fs::rename(old, cur);
        }
    };
    for (name, staged) in files {
        let cur = dir.join(name);
        let old = dir.join(format!("{}.old", name));
        let _ = fs::remove_file(&old);
        if cur.exists() {
            if let Err(e) = fs::rename(&cur, &old) {
                undo(&done);
                return Err(format!("cannot replace {} ({}) — is the app in a protected folder?", name, e));
            }
        }
        done.push((cur.clone(), old.clone()));
        if fs::rename(staged, &cur).is_err() {
            if let Err(e) = fs::copy(staged, &cur) {
                undo(&done);
                return Err(e.to_string());
            }
        }
    }
    Ok(())
}
/// Remove `*.old` files left by the previous self-update.
pub fn clean_old(dir: &Path) {
    for n in APP_FILES {
        let _ = fs::remove_file(dir.join(format!("{}.old", n)));
    }
}

fn write(p: &Path, b: &[u8]) -> Result<(), String> {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    fs::write(p, b).map_err(|e| e.to_string())
}
fn get(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut buf = vec![];
    resp.into_body().into_reader().take(MAX_FILE + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.len() as u64 > MAX_FILE {
        return Err("file too large".into());
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(ver_key("2026.11.16-2") > ver_key("2026.11.16-1"));
        assert!(ver_key("2026.12.1-1") > ver_key("2026.11.30-9"));
    }
    #[test]
    fn paths() {
        for bad in ["", "/etc/x", "..", "a/../b", "C:/x", "a\\b", "a//b", "./a"] {
            assert!(!safe_rel(bad), "{}", bad);
        }
        assert!(safe_rel("pkg/studyhub_web.js"));
    }
}
