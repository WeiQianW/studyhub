//! End-to-end: serve a "newer version" over a local HTTP server and check install / reject paths.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use studyhub_update::*;

fn manifest(ver: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
    let fs: Vec<_> = files.iter().map(|(p, b)| serde_json::json!({"path": p, "sha256": sha_hex(b), "size": b.len()})).collect();
    serde_json::to_vec(&serde_json::json!({"app": "studyhub", "version": ver, "files": fs})).unwrap()
}
fn server(files: Arc<Mutex<HashMap<String, Vec<u8>>>>) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let mut s = s.unwrap();
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = req.split_whitespace().nth(1).unwrap_or("/").trim_start_matches('/').to_string();
            let body = files.lock().unwrap().get(&path).cloned();
            let resp = match body {
                Some(b) => [format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", b.len()).into_bytes(), b].concat(),
                None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
            };
            let _ = s.write_all(&resp);
        }
    });
    format!("http://{}/", addr)
}

#[test]
fn update_lifecycle() {
    let old_index: &'static [u8] = b"<p>v1</p>";
    let old_content: &'static [u8] = b"{\"v\":1}";
    let m1: &'static [u8] = Box::leak(manifest("2026.11.16-1", &[("index.html", old_index), ("content.json", old_content)]).into_boxed_slice());
    let bundle: Bundle = Box::leak(vec![("index.html", old_index), ("content.json", old_content), ("manifest.json", m1)].into_boxed_slice());
    let dir = std::env::temp_dir().join(format!("shupd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let st = Store { dir: dir.clone(), bundle };
    assert_eq!(st.serve("index.html").unwrap(), old_index);

    let files = Arc::new(Mutex::new(HashMap::new()));
    let base = server(files.clone());
    // offline / nothing published: an error, nothing changes
    assert!(st.check(&base, true).is_err());
    // same version published: no update
    files.lock().unwrap().insert("manifest.json".into(), m1.to_vec());
    assert_eq!(st.check(&base, true).unwrap(), None);
    // newer version, but a tampered file: rejected, still on v1
    let new_content = b"{\"v\":2}".to_vec();
    let m2 = manifest("2026.11.17-1", &[("index.html", old_index), ("content.json", &new_content)]);
    files.lock().unwrap().insert("manifest.json".into(), m2.clone());
    files.lock().unwrap().insert("content.json".into(), b"{\"v\":666}".to_vec());
    assert!(st.check(&base, true).is_err());
    assert_eq!(st.serve("content.json").unwrap(), old_content);
    // correct file: installed; unchanged index.html is reused from the bundle (not downloaded)
    files.lock().unwrap().insert("content.json".into(), new_content.clone());
    assert_eq!(st.check(&base, true).unwrap().as_deref(), Some("2026.11.17-1"));
    assert_eq!(st.serve("content.json").unwrap(), new_content);
    assert_eq!(st.active_version(), "2026.11.17-1");
    // a file damaged on disk afterwards is not served
    std::fs::write(dir.join("current/content.json"), b"{\"v\":9}").unwrap();
    assert!(st.serve("content.json").is_none());
    // path traversal in a manifest is refused
    let evil = serde_json::to_vec(&serde_json::json!({"app":"studyhub","version":"2027.1.1-1","files":[{"path":"../x","sha256":"00","size":1},{"path":"index.html","sha256":sha_hex(old_index),"size":old_index.len()}]})).unwrap();
    files.lock().unwrap().insert("manifest.json".into(), evil);
    assert!(st.check(&base, true).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
