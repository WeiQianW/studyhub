//! Mechatronics Study Hub — desktop app.
//!
//! * Works fully offline: the whole site (Rust/WASM program + content) is built into the .exe.
//! * When online it checks the update address for a newer `manifest.json`, downloads the changed
//!   files, verifies every file's SHA-256 against the manifest, and switches over atomically.
//!   The next launch (or the Reload button in the banner) uses the new version.
//! * Pages are served from the app's own `studyhub://` origin with a strict Content-Security-Policy;
//!   navigation to any other site is blocked.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use studyhub_update::{mime, Store};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::http::{header, Request, Response, StatusCode};
use wry::{WebContext, WebViewBuilder};

/// Where updates are published (a static folder containing manifest.json and the files it lists).
/// Can be overridden with an `update-url.txt` file in the app data folder.
const DEFAULT_UPDATE_URL: &str = "https://weiqianw.github.io/studyhub/";
const CHECK_EVERY: Duration = Duration::from_secs(15 * 60);

macro_rules! bundled {
    ($($p:literal),*) => { &[$(($p, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dist/web/", $p)) as &[u8])),*] };
}
static BUNDLE: &[(&str, &[u8])] = bundled!("index.html", "pkg/studyhub_web.js", "pkg/studyhub_web_bg.wasm", "content.json", "manifest.json");

fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("MechatronicsStudyHub")
}
fn store() -> Store {
    Store { dir: data_dir(), bundle: BUNDLE }
}
fn update_url() -> String {
    let mut u = fs::read_to_string(data_dir().join("update-url.txt")).ok().map(|s| s.trim().to_string()).filter(|s| s.starts_with("https://")).unwrap_or_else(|| DEFAULT_UPDATE_URL.to_string());
    if !u.ends_with('/') {
        u.push('/');
    }
    u
}
fn log(msg: &str) {
    let p = data_dir().join("update.log");
    let prev = fs::read_to_string(&p).unwrap_or_default();
    let mut lines: Vec<&str> = prev.lines().collect();
    if lines.len() > 50 {
        lines.drain(..lines.len() - 50);
    }
    let _ = fs::write(&p, format!("{}\n{}", lines.join("\n"), msg));
}

#[derive(Debug)]
enum UserEvent {
    Updated(String),
    NewApp(String),
}

const DOWNLOAD_PAGE: &str = "https://weiqianw.github.io/studyhub/download/";
const APP_BANNER_JS: &str = r#"(function(v){if(document.getElementById('shApp'))return;var b=document.createElement('div');b.id='shApp';b.style.cssText='position:fixed;left:50%;bottom:70px;transform:translateX(-50%);z-index:99999;background:var(--card,#222);color:var(--fg,#fff);border:1px solid var(--bolt,#3aa0ff);border-radius:12px;padding:10px 14px;box-shadow:0 8px 30px rgba(0,0,0,.4);font:14px system-ui;display:flex;gap:12px;align-items:center';var s=document.createElement('span');s.textContent='New app version '+v+' available · 有新版本应用';var r=document.createElement('button');r.textContent='Download · 下载';r.className='btn sm';r.onclick=function(){window.ipc.postMessage('open-download')};var x=document.createElement('button');x.textContent='✕';x.className='chipbtn';x.onclick=function(){b.remove()};b.appendChild(s);b.appendChild(r);b.appendChild(x);document.body.appendChild(b);})"#;

fn open_in_browser(url: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).creation_flags(0x08000000).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

const BANNER_JS: &str = r#"(function(v){var b=document.getElementById('shUpd');if(!b){b=document.createElement('div');b.id='shUpd';b.style.cssText='position:fixed;left:50%;bottom:18px;transform:translateX(-50%);z-index:99999;background:var(--card,#222);color:var(--fg,#fff);border:1px solid var(--accent,#f80);border-radius:12px;padding:10px 14px;box-shadow:0 8px 30px rgba(0,0,0,.4);font:14px system-ui;display:flex;gap:12px;align-items:center';document.body.appendChild(b);}b.textContent='';var s=document.createElement('span');s.textContent='Update '+v+' downloaded · 新版本已下载';var r=document.createElement('button');r.textContent='Reload · 重新载入';r.className='btn sm';r.onclick=function(){location.reload()};b.appendChild(s);b.appendChild(r);})"#;

fn main() {
    let _ = fs::create_dir_all(data_dir());
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || loop {
        match store().check_full(&update_url(), false) {
            Ok((installed, desk)) => {
                if let Some(v) = installed {
                    log(&format!("installed {}", v));
                    let _ = proxy.send_event(UserEvent::Updated(v));
                }
                if let Some(d) = desk {
                    if studyhub_update::ver_key(&d) > studyhub_update::ver_key(env!("CARGO_PKG_VERSION")) {
                        let _ = proxy.send_event(UserEvent::NewApp(d));
                    }
                }
            }
            Err(e) => log(&format!("check failed (offline?): {}", e)),
        }
        std::thread::sleep(CHECK_EVERY);
    });

    let window = WindowBuilder::new().with_title("Mechatronics Study Hub").with_inner_size(tao::dpi::LogicalSize::new(1280.0, 860.0)).build(&event_loop).expect("window");
    let mut ctx = WebContext::new(Some(data_dir().join("webview")));
    let webview = WebViewBuilder::new_with_web_context(&mut ctx)
        .with_custom_protocol("studyhub".into(), move |_id, req: Request<Vec<u8>>| {
            let path = req.uri().path().trim_start_matches('/').to_string();
            let path = if path.is_empty() { "index.html".to_string() } else { path };
            match store().serve(&path) {
                Some(body) => Response::builder()
                    .header(header::CONTENT_TYPE, mime(&path))
                    .header(header::CACHE_CONTROL, "no-cache")
                    .header("X-Content-Type-Options", "nosniff")
                    .header(
                        "Content-Security-Policy",
                        "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com data:; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'",
                    )
                    .body(Cow::Owned(body))
                    .unwrap(),
                None => Response::builder().status(StatusCode::NOT_FOUND).body(Cow::Borrowed(&b"not found"[..])).unwrap(),
            }
        })
        .with_navigation_handler(|url| url.starts_with("studyhub://") || url.starts_with("http://studyhub.localhost") || url.starts_with("https://studyhub.localhost"))
        .with_new_window_req_handler(|_url, _features| wry::NewWindowResponse::Deny)
        .with_initialization_script("window.SH_DESKTOP=true;")
        .with_ipc_handler(|req| {
            if req.body() == "open-download" {
                open_in_browser(DOWNLOAD_PAGE);
            }
        })
        .with_devtools(false)
        .with_url(if cfg!(windows) { "http://studyhub.localhost/index.html" } else { "studyhub://localhost/index.html" })
        .build(&window)
        .expect("webview");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Updated(v)) => {
                let v = v.replace(['\'', '\\', '<'], "");
                let _ = webview.evaluate_script(&format!("{}('{}')", BANNER_JS, v));
            }
            Event::UserEvent(UserEvent::NewApp(v)) => {
                let v = v.replace(['\'', '\\', '<'], "");
                let _ = webview.evaluate_script(&format!("{}('{}')", APP_BANNER_JS, v));
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

