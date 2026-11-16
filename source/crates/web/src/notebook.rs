//! Notebook: highlights and personal notes, anchored in the text; optional sharing through the
//! page's database when it runs as a published page (`window.claude`), browser-only otherwise.
use crate::util::*;
use crate::*;
use indexmap::IndexMap;
use js_sys::{Array, Function, Promise, Reflect};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::Element;

const HLC: [(&str, &str); 4] = [("yellow", "#f5c518"), ("green", "#2fcf86"), ("blue", "#3aa0ff"), ("pink", "#ff6fae")];
fn hlc(c: &str) -> &'static str {
    HLC.iter().find(|x| x.0 == c).map(|x| x.1).unwrap_or(HLC[0].1)
}

#[derive(Clone, Debug)]
pub struct Item {
    id: String,
    topic: String,
    sec: f64,
    quote: String,
    occ: f64,
    color: String,
    note: String,
    ts: f64,
    vis: String,
    local: bool,
    uid: Option<String>,
}
impl Item {
    fn from(v: &Value, id: &str) -> Item {
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let n = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
        Item {
            id: id.to_string(),
            topic: s("topic"),
            sec: v.get("sec").and_then(|x| x.as_f64().or_else(|| x.as_str().map(parse_float))).unwrap_or(0.0),
            quote: s("quote"),
            occ: n("occ"),
            color: v.get("color").and_then(|x| x.as_str()).unwrap_or("yellow").to_string(),
            note: s("note"),
            ts: n("ts"),
            vis: "private".into(),
            local: false,
            uid: None,
        }
    }
    fn body(&self) -> Value {
        let ts = if self.ts != 0.0 { self.ts } else { date_now() };
        json!({"topic": self.topic, "sec": num_json(self.sec), "quote": self.quote, "occ": num_json(self.occ), "color": if self.color.is_empty() {"yellow"} else {&self.color}, "note": self.note, "ts": num_json(ts)})
    }
}
fn num_json(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        Value::from(v as i64)
    } else {
        Value::from(v)
    }
}

#[derive(Clone)]
struct Pending {
    topic: String,
    sec: f64,
    quote: String,
    occ: f64,
}

pub struct NbSt {
    mode: &'static str,
    uid: Option<String>,
    me: Value,
    prv: IndexMap<String, Item>,
    shar: IndexMap<String, Item>,
    local: IndexMap<String, Item>,
    others: IndexMap<String, IndexMap<String, Item>>,
    show_others: bool,
    def_vis: String,
    scope: String,
    pub open: bool,
    editing: Option<String>,
    draft: String,
    pending: Option<Pending>,
    confirm_del: Option<String>,
    marker_n: f64,
    prof: serde_json::Map<String, Value>,
    prof_busy: bool,
    open_box: HashMap<String, bool>,
    focus_next: bool,
    queued: bool,
    marker_t: i32,
    sel_t: i32,
    toast_t: i32,
    tb_lang: String,
}
impl NbSt {
    pub fn load() -> NbSt {
        NbSt {
            mode: "init",
            uid: None,
            me: Value::Null,
            prv: IndexMap::new(),
            shar: IndexMap::new(),
            local: IndexMap::new(),
            others: IndexMap::new(),
            show_others: store_get("nbShowOthers").and_then(|v| v.as_bool()).unwrap_or(true),
            def_vis: store_get("nbDefVis").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "private".into()),
            scope: "topic".into(),
            open: false,
            editing: None,
            draft: String::new(),
            pending: None,
            confirm_del: None,
            marker_n: -1.0,
            prof: Default::default(),
            prof_busy: false,
            open_box: HashMap::new(),
            focus_next: false,
            queued: false,
            marker_t: 0,
            sel_t: 0,
            toast_t: 0,
            tb_lang: String::new(),
        }
    }
    fn mine(&self) -> IndexMap<String, Item> {
        let mut m = IndexMap::new();
        if self.mode != "db" {
            for (k, v) in &self.local {
                let mut it = v.clone();
                it.vis = "private".into();
                it.local = true;
                m.insert(k.clone(), it);
            }
        } else {
            for (k, v) in &self.prv {
                let mut it = v.clone();
                it.vis = "private".into();
                m.insert(k.clone(), it);
            }
            for (k, v) in &self.shar {
                let mut it = v.clone();
                it.vis = "shared".into();
                m.insert(k.clone(), it);
            }
        }
        m
    }
}

thread_local! {
    static DB: RefCell<Option<JsValue>> = RefCell::new(None);
    static USER: RefCell<Option<JsValue>> = RefCell::new(None);
    static SUBS: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
}

// ---------- JS interop ----------
fn jcall(o: &JsValue, m: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let f: Function = Reflect::get(o, &JsValue::from_str(m))?.dyn_into()?;
    let a = Array::new();
    for x in args {
        a.push(x);
    }
    f.apply(o, &a)
}
async fn jawait(v: JsValue) -> Result<JsValue, JsValue> {
    JsFuture::from(Promise::resolve(&v)).await
}
fn to_value(v: &JsValue) -> Value {
    js_sys::JSON::stringify(v).ok().and_then(|s| s.as_string()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}
fn to_js(v: &Value) -> JsValue {
    js_sys::JSON::parse(&v.to_string()).unwrap_or(JsValue::NULL)
}
fn snapshot_docs(s: &JsValue) -> Vec<(String, Value)> {
    let docs = Reflect::get(s, &"docs".into()).unwrap_or(JsValue::UNDEFINED);
    let mut out = vec![];
    if let Ok(arr) = docs.dyn_into::<Array>() {
        for d in arr.iter() {
            let id = Reflect::get(&d, &"id".into()).ok().and_then(|x| x.as_string()).unwrap_or_default();
            let data = jcall(&d, "data", &[]).map(|x| to_value(&x)).unwrap_or(Value::Null);
            out.push((id, data));
        }
    }
    out
}
fn db() -> Option<JsValue> {
    DB.with(|d| d.borrow().clone())
}
fn on_snapshot(target: &JsValue, f: impl FnMut(JsValue) + 'static) -> JsValue {
    let cb = Closure::<dyn FnMut(JsValue)>::new(f);
    let errcb = Closure::<dyn FnMut(JsValue)>::new(|_e: JsValue| {});
    let r = jcall(target, "onSnapshot", &[cb.as_ref().clone(), errcb.as_ref().clone()]).unwrap_or(JsValue::UNDEFINED);
    cb.forget();
    errcb.forget();
    r
}
fn doc_ref(vis: &str, id: &str) -> Result<JsValue, JsValue> {
    let d = db().ok_or(JsValue::NULL)?;
    let uid = with(|s| s.nb.uid.clone()).unwrap_or_default();
    if vis == "shared" {
        let col = jcall(&d, "collection", &[JsValue::from_str(&format!("shared/{}/items", uid))])?;
        jcall(&col, "doc", &[JsValue::from_str(id)])
    } else {
        let col = jcall(&d, "collection", &[JsValue::from_str(&format!("data/users/{}", uid))])?;
        jcall(&col, "doc", &[JsValue::from_str(id)])
    }
}

// ---------- local storage ----------
fn save_local() {
    let v = with(|s| {
        Value::Array(
            s.nb.local
                .values()
                .map(|it| {
                    let mut b = it.body();
                    b.as_object_mut().unwrap().insert("id".into(), Value::String(it.id.clone()));
                    b
                })
                .collect(),
        )
    });
    ls_set("dpsh_nbLocal", &v.to_string());
}
fn load_local() {
    let arr = ls_get("dpsh_nbLocal").and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    with(|s| {
        for v in arr {
            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let it = Item::from(&v, &id);
            s.nb.local.insert(id, it);
        }
    });
}
fn new_id() -> String {
    let t = js_sys::Number::from(date_now()).to_string(36).map(String::from).unwrap_or_default();
    let r = js_sys::Number::from(random()).to_string(36).map(String::from).unwrap_or_default();
    let tail: String = r.chars().skip(2).take(5).collect();
    format!("h{}{}", t, tail)
}

fn key_now() -> Option<String> {
    with(|s| match s.tab.as_str() {
        "formulas" => Some("fx".to_string()),
        "learn" => Some(s.topic.clone()),
        _ => None,
    })
}
fn topic_label(k: &str) -> String {
    if k == "fx" {
        return lp("Formula Sheet", "公式表");
    }
    match c().topic(k) {
        Some(t) => format!("{} · {}", wk(t), ttp(t)),
        None => k.to_string(),
    }
}

// ---------- writes ----------
fn write(item: Item, vis: String) {
    spawn_local(async move {
        let body = item.body();
        let mode = with(|s| s.nb.mode);
        if mode != "db" {
            with(|s| {
                let mut it = Item::from(&body, &item.id);
                it.local = true;
                s.nb.local.insert(item.id.clone(), it);
            });
            save_local();
            changed();
            return;
        }
        let r = match doc_ref(&vis, &item.id) {
            Ok(r) => jawait(jcall(&r, "set", &[to_js(&body)]).unwrap_or(JsValue::UNDEFINED)).await,
            Err(e) => Err(e),
        };
        match r {
            Ok(_) => {
                if vis == "shared" {
                    marker_soon();
                }
            }
            Err(e) => {
                let code = Reflect::get(&e, &"code".into()).ok().and_then(|x| x.as_string()).unwrap_or_default();
                if code == "invalid_argument" {
                    go_local();
                    with(|s| {
                        s.nb.local.insert(item.id.clone(), Item::from(&body, &item.id));
                    });
                    save_local();
                    changed();
                } else {
                    toast(&lp("Could not save. Try again in a moment.", "保存失败，请稍后再试。"));
                }
            }
        }
    });
}
fn delete(item: Item) {
    spawn_local(async move {
        let mode = with(|s| s.nb.mode);
        if mode != "db" || item.local {
            with(|s| {
                s.nb.local.shift_remove(&item.id);
            });
            save_local();
            changed();
            return;
        }
        let r = match doc_ref(&item.vis, &item.id) {
            Ok(r) => jawait(jcall(&r, "delete", &[]).unwrap_or(JsValue::UNDEFINED)).await,
            Err(e) => Err(e),
        };
        match r {
            Ok(_) => {
                if item.vis == "shared" {
                    marker_soon();
                }
            }
            Err(_) => toast(&lp("Could not delete. Try again in a moment.", "删除失败，请稍后再试。")),
        }
    });
}
fn mv(item: Item, vis: String) {
    if with(|s| s.nb.mode) != "db" || item.vis == vis {
        return;
    }
    spawn_local(async move {
        let body = item.body();
        if let Ok(r) = doc_ref(&vis, &item.id) {
            if jawait(jcall(&r, "set", &[to_js(&body)]).unwrap_or(JsValue::UNDEFINED)).await.is_ok() && vis == "shared" {
                marker_soon();
            }
        }
        if let Ok(r) = doc_ref(&item.vis, &item.id) {
            let _ = jawait(jcall(&r, "delete", &[]).unwrap_or(JsValue::UNDEFINED)).await;
        }
        marker_soon();
    });
}
fn marker_soon() {
    let t = with(|s| s.nb.marker_t);
    clear_timeout(t);
    let id = timeout(900, || {
        spawn_local(async {
            let (ok, uid, n, mk) = with(|s| (s.nb.mode == "db", s.nb.uid.clone(), s.nb.shar.len() as f64, s.nb.marker_n));
            let d = match db() {
                Some(d) => d,
                None => return,
            };
            if !ok || uid.is_none() || n == mk {
                return;
            }
            if let Ok(r) = jcall(&d, "doc", &[JsValue::from_str(&format!("shared/{}", uid.unwrap()))]) {
                if jawait(jcall(&r, "set", &[to_js(&json!({"n": n as i64, "ts": date_now() as i64}))]).unwrap_or(JsValue::UNDEFINED)).await.is_ok() {
                    with(|s| s.nb.marker_n = n);
                }
            }
        });
    });
    with(|s| s.nb.marker_t = id);
}
fn go_local() {
    if with(|s| s.nb.mode) != "db" {
        return;
    }
    with(|s| {
        s.nb.mode = "local-ro";
        s.nb.prv.clear();
        s.nb.shar.clear();
    });
    changed();
}
fn optimistic(it: &Item, vis: &str) {
    with(|s| {
        if s.nb.mode == "db" {
            let m = if vis == "shared" { &mut s.nb.shar } else { &mut s.nb.prv };
            m.insert(it.id.clone(), it.clone());
        }
    });
    changed();
}

pub fn changed() {
    if with(|s| s.nb.queued) {
        return;
    }
    with(|s| s.nb.queued = true);
    raf(|| {
        with(|s| s.nb.queued = false);
        apply_marks();
        if with(|s| s.nb.open) {
            render();
        } else {
            fab();
        }
    });
}

pub fn after_render() {
    if with(|s| s.nb.tb_lang.clone()) != lang().code() {
        tb_render();
    }
    let topic = with(|s| s.topic.clone());
    for (i, el) in qsa(&app(), ".reader section").iter().enumerate() {
        set_attr(el, "data-hk", &topic);
        set_attr(el, "data-sec", &i.to_string());
    }
    hide_tb();
    apply_marks();
    render();
}

// ---------- data subscriptions ----------
pub fn init() {
    load_local();
    spawn_local(async {
        let c = Reflect::get(&win(), &"claude".into()).unwrap_or(JsValue::UNDEFINED);
        let use_fn = if c.is_object() { Reflect::get(&c, &"use".into()).unwrap_or(JsValue::UNDEFINED) } else { JsValue::UNDEFINED };
        if !use_fn.is_function() {
            with(|s| s.nb.mode = "local");
            changed();
            return;
        }
        let dbp = jcall(&c, "use", &[JsValue::from_str("db")]);
        let up = jcall(&c, "use", &[JsValue::from_str("user")]);
        let (mut d, mut u) = (JsValue::NULL, JsValue::NULL);
        if let (Ok(dbp), Ok(up)) = (dbp, up) {
            let all = Array::of2(&dbp, &up);
            if let Ok(r) = JsFuture::from(Promise::all(&all)).await {
                let r: Array = r.unchecked_into();
                d = r.get(0);
                u = r.get(1);
            }
        }
        let have_user = !u.is_null() && !u.is_undefined();
        USER.with(|x| *x.borrow_mut() = if have_user { Some(u.clone()) } else { None });
        if have_user {
            if let Ok(p) = jcall(&u, "me", &[]) {
                if let Ok(me) = jawait(p).await {
                    with(|s| s.nb.me = to_value(&me));
                }
            }
        }
        let uid = with(|s| s.nb.me.get("id").and_then(|x| x.as_str().map(String::from)));
        let have_db = !d.is_null() && !d.is_undefined();
        if !have_db || uid.is_none() {
            DB.with(|x| *x.borrow_mut() = if have_db { Some(d.clone()) } else { None });
            with(|s| s.nb.mode = "local");
            if have_db {
                watch_sharers();
            }
            changed();
            return;
        }
        let uid = uid.unwrap();
        DB.with(|x| *x.borrow_mut() = Some(d.clone()));
        with(|s| {
            s.nb.uid = Some(uid.clone());
            s.nb.mode = "db";
        });
        if let Ok(col) = jcall(&d, "collection", &[JsValue::from_str(&format!("data/users/{}", uid))]) {
            on_snapshot(&col, |snap| {
                let docs = snapshot_docs(&snap);
                with(|s| s.nb.prv = docs.iter().map(|(id, v)| (id.clone(), Item::from(v, id))).collect());
                changed();
            });
        }
        if let Ok(col) = jcall(&d, "collection", &[JsValue::from_str(&format!("shared/{}/items", uid))]) {
            on_snapshot(&col, |snap| {
                let docs = snapshot_docs(&snap);
                with(|s| s.nb.shar = docs.iter().map(|(id, v)| (id.clone(), Item::from(v, id))).collect());
                changed();
            });
        }
        if let Ok(r) = jcall(&d, "doc", &[JsValue::from_str(&format!("shared/{}", uid))]) {
            if let Ok(p) = jcall(&r, "get", &[]) {
                if let Ok(snap) = jawait(p).await {
                    let exists = Reflect::get(&snap, &"exists".into()).ok().map(|x| x.is_truthy()).unwrap_or(false);
                    let n = if exists { jcall(&snap, "data", &[]).map(|x| to_value(&x)).ok().and_then(|v| v.get("n").and_then(|n| n.as_f64())).unwrap_or(0.0) } else { 0.0 };
                    with(|s| s.nb.marker_n = n);
                }
            }
        }
        watch_sharers();
    });
}
fn watch_sharers() {
    let d = match db() {
        Some(d) => d,
        None => return,
    };
    let col = match jcall(&d, "collection", &[JsValue::from_str("shared")]).and_then(|c| jcall(&c, "limit", &[JsValue::from_f64(60.0)])) {
        Ok(c) => c,
        Err(_) => return,
    };
    on_snapshot(&col, move |snap| {
        let me = with(|s| s.nb.uid.clone()).unwrap_or_default();
        let ids: Vec<String> = snapshot_docs(&snap)
            .into_iter()
            .filter(|(id, v)| *id != me && v.get("n").and_then(|n| n.as_f64()).unwrap_or(0.0) > 0.0)
            .map(|(id, _)| id)
            .collect();
        // drop subscriptions for people who no longer share
        let gone: Vec<String> = SUBS.with(|m| m.borrow().keys().filter(|k| !ids.contains(k)).cloned().collect());
        for id in gone {
            if let Some(un) = SUBS.with(|m| m.borrow_mut().remove(&id)) {
                if un.is_function() {
                    let _ = un.unchecked_ref::<Function>().call0(&JsValue::NULL);
                }
            }
            with(|s| {
                s.nb.others.shift_remove(&id);
            });
        }
        if let Some(d) = db() {
            for id in ids.iter().take(40) {
                if SUBS.with(|m| m.borrow().contains_key(id)) {
                    continue;
                }
                let q = jcall(&d, "collection", &[JsValue::from_str(&format!("shared/{}/items", id))]).and_then(|c| jcall(&c, "limit", &[JsValue::from_f64(800.0)]));
                if let Ok(q) = q {
                    let id2 = id.clone();
                    let un = on_snapshot(&q, move |snap| {
                        let docs = snapshot_docs(&snap);
                        let m: IndexMap<String, Item> = docs
                            .iter()
                            .map(|(did, v)| {
                                let mut it = Item::from(v, did);
                                it.uid = Some(id2.clone());
                                (did.clone(), it)
                            })
                            .collect();
                        with(|s| {
                            s.nb.others.insert(id2.clone(), m);
                        });
                        changed();
                    });
                    SUBS.with(|m| m.borrow_mut().insert(id.clone(), un));
                }
            }
        }
        changed();
    });
}

// ---------- anchoring marks in the text ----------
fn u16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn index_of(hay: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(from.min(hay.len()));
    }
    if needle.len() > hay.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}
fn nth_index(hay: &[u16], needle: &[u16], n: usize) -> Option<usize> {
    let mut i: i64 = -1;
    for _ in 0..=n {
        match index_of(hay, needle, (i + 1) as usize) {
            Some(j) => i = j as i64,
            None => return None,
        }
    }
    Some(i as usize)
}
fn unwrap_marks() {
    for m in qsa(&app(), "mark.hl") {
        if let Some(p) = m.parent_node() {
            while let Some(ch) = m.first_child() {
                let _ = p.insert_before(&ch, Some(&m));
            }
            let _ = p.remove_child(&m);
            p.normalize();
        }
    }
}
fn wrap_range(host: &Element, start: usize, end: usize, it: &Item) {
    let w = match doc().create_tree_walker_with_what_to_show(host, 4) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut pos = 0usize;
    let mut parts: Vec<(web_sys::Text, usize, usize)> = vec![];
    while let Ok(Some(n)) = w.next_node() {
        let len = n.node_value().map(|v| v.encode_utf16().count()).unwrap_or(0);
        let a = start.max(pos);
        let b = end.min(pos + len);
        if a < b {
            parts.push((n.unchecked_into(), a - pos, b - pos));
        }
        pos += len;
        if pos >= end {
            break;
        }
    }
    for (node, a, b) in parts {
        if let Some(pe) = node.parent_element() {
            if closest(&pe, "svg,textarea,select,option").is_some() {
                continue;
            }
        }
        let mut t = node;
        if a > 0 {
            t = match t.split_text(a as u32) {
                Ok(x) => x,
                Err(_) => continue,
            };
        }
        let tl = t.length() as usize;
        if b - a < tl {
            let _ = t.split_text((b - a) as u32);
        }
        let m = doc().create_element("mark").unwrap();
        m.set_class_name(&format!("hl{}{}", if it.uid.is_some() { " other" } else { "" }, if !it.note.is_empty() { " has-note" } else { "" }));
        set_attr(&m, "data-hid", &it.id);
        if let Some(u) = &it.uid {
            set_attr(&m, "data-uid", u);
        }
        let _ = hel(&m).style().set_property("--c", hlc(&it.color));
        set_attr(&m, "title", &if !it.note.is_empty() { u16_prefix(&it.note, 140) } else { String::new() });
        if let Some(p) = t.parent_node() {
            let _ = p.insert_before(&m, Some(&t));
            let _ = m.append_child(&t);
        }
    }
}
fn u16_prefix(s: &str, n: usize) -> String {
    let v = u16(s);
    String::from_utf16_lossy(&v[..n.min(v.len())])
}
pub fn apply_marks() {
    i18n::flush();
    unwrap_marks();
    let hosts = qsa(&app(), "[data-hk]");
    if hosts.is_empty() {
        return;
    }
    let items: Vec<Item> = with(|s| {
        let mut v: Vec<Item> = s.nb.mine().into_values().collect();
        if s.nb.show_others {
            for m in s.nb.others.values() {
                v.extend(m.values().cloned());
            }
        }
        v
    });
    for h in hosts {
        let key = attr(&h, "data-hk");
        let sec = parse_float(&attr(&h, "data-sec"));
        let its: Vec<&Item> = items.iter().filter(|it| it.topic == key && it.sec == sec && !it.quote.is_empty()).collect();
        for it in its {
            let txt = u16(&h.text_content().unwrap_or_default());
            let q = u16(&it.quote);
            let i = nth_index(&txt, &q, it.occ.max(0.0) as usize).or_else(|| index_of(&txt, &q, 0));
            if let Some(i) = i {
                wrap_range(&h, i, i + q.len(), it);
            }
        }
    }
    i18n::discard();
}

// ---------- selection toolbar ----------
fn tb() -> Element {
    by_id("nbTb").unwrap()
}
fn tb_render() {
    let h: String = HLC.iter().map(|(c, v)| format!("<button class=\"sw8\" data-c=\"{}\" style=\"--c:{}\" title=\"Highlight\" aria-label=\"Highlight {}\"></button>", c, v, c)).collect::<String>()
        + &format!("<button class=\"nbtn\" data-act=\"note\">{}</button>", lp("Add note", "添加笔记"));
    let t = tb();
    set_html(&t, &h);
    set_attr(&t, "data-lang", lang().code());
    with(|s| s.nb.tb_lang = lang().code().to_string());
}
fn hide_tb() {
    if let Some(t) = by_id("nbTb") {
        set_hidden(&t, true);
    }
    with(|s| s.nb.pending = None);
}
fn on_selection() {
    let sel = match win().get_selection().ok().flatten() {
        Some(s) => s,
        None => return hide_tb(),
    };
    if sel.is_collapsed() || sel.range_count() == 0 {
        return hide_tb();
    }
    let r = match sel.get_range_at(0) {
        Ok(r) => r,
        Err(_) => return hide_tb(),
    };
    let cac = match r.common_ancestor_container() {
        Ok(n) => n,
        Err(_) => return hide_tb(),
    };
    let anc: Option<Element> = if cac.node_type() == 1 { Some(cac.unchecked_into()) } else { cac.parent_element() };
    let anc = match anc {
        Some(a) => a,
        None => return hide_tb(),
    };
    let host = match closest(&anc, "[data-hk]") {
        Some(h) if app().contains(Some(&h)) && closest(&anc, "input,textarea,select,button.blank").is_none() => h,
        _ => return hide_tb(),
    };
    let raw = sel.to_string().as_string().unwrap_or_default();
    let quote = js_trim(&raw).to_string();
    let ql = quote.encode_utf16().count();
    if ql < 2 || ql > 600 {
        return hide_tb();
    }
    let pre = doc().create_range().unwrap();
    let _ = pre.set_start(&host, 0);
    let _ = pre.set_end(&r.start_container().unwrap(), r.start_offset().unwrap_or(0));
    let lead = raw.encode_utf16().count() - raw.trim_start_matches(|c: char| is_js_space(c)).encode_utf16().count();
    let start = pre.to_string().as_string().unwrap_or_default().encode_utf16().count() + lead;
    let txt = u16(&host.text_content().unwrap_or_default());
    let qv = u16(&quote);
    let mut occ = 0;
    let mut i: i64 = -1;
    while let Some(j) = index_of(&txt, &qv, (i + 1) as usize) {
        if j >= start {
            break;
        }
        i = j as i64;
        occ += 1;
    }
    with(|s| s.nb.pending = Some(Pending { topic: attr(&host, "data-hk"), sec: parse_float(&attr(&host, "data-sec")), quote: quote.clone(), occ: occ as f64 }));
    let rect = r.get_bounding_client_rect();
    let t = tb();
    set_hidden(&t, false);
    let he = hel(&t);
    let w = if he.offset_width() > 0 { he.offset_width() as f64 } else { 240.0 };
    let h = if he.offset_height() > 0 { he.offset_height() as f64 } else { 40.0 };
    let touch = win().match_media("(pointer:coarse)").ok().flatten().map(|m| m.matches()).unwrap_or(false);
    let mut top = if touch { rect.bottom() + 10.0 } else { rect.top() - h - 10.0 };
    if top < 70.0 {
        top = rect.bottom() + 10.0;
    }
    let iw = win().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let mut left = rect.left() + rect.width() / 2.0 - w / 2.0;
    left = left.min(iw - w - 8.0).max(8.0);
    let st = he.style();
    let _ = st.set_property("top", &format!("{}px", num_str(top + scroll_y())));
    let _ = st.set_property("left", &format!("{}px", num_str(left + win().scroll_x().unwrap_or(0.0))));
}

fn tb_click(t: &Element) {
    let b = match closest(t, "button") {
        Some(b) => b,
        None => return,
    };
    let p = match with(|s| s.nb.pending.clone()) {
        Some(p) => p,
        None => return,
    };
    let it = Item {
        id: new_id(),
        topic: p.topic,
        sec: p.sec,
        quote: p.quote,
        occ: p.occ,
        color: dataset(&b, "c").unwrap_or_else(|| "yellow".into()),
        note: String::new(),
        ts: date_now(),
        vis: "private".into(),
        local: false,
        uid: None,
    };
    if let Ok(Some(sel)) = win().get_selection() {
        let _ = sel.remove_all_ranges();
    }
    hide_tb();
    let vis = with(|s| if s.nb.mode == "db" { s.nb.def_vis.clone() } else { "private".into() });
    if dataset(&b, "act").as_deref() == Some("note") {
        with(|s| {
            s.nb.editing = Some(it.id.clone());
            s.nb.draft.clear();
            s.nb.open = true;
            s.nb.scope = "topic".into();
        });
    }
    let mut it2 = it.clone();
    it2.vis = vis.clone();
    optimistic(&it2, &vis);
    write(it, vis);
}

// ---------- drawer ----------
fn nb_el() -> Element {
    by_id("nb").unwrap()
}
fn fab() {
    let f = match by_id("nbFab") {
        Some(f) => f,
        None => return,
    };
    let k = key_now();
    let open = with(|s| s.nb.open);
    set_hidden(&f, k.is_none() || open);
    let k = match k {
        Some(k) => k,
        None => return,
    };
    let n = with(|s| s.nb.mine().values().filter(|i| i.topic == k).count());
    set_html(&f, &format!("<span>Notebook</span><b>{}</b>", n));
    i18n::translate(&f);
}

fn fmt_date(ts: f64) -> String {
    let d = js_sys::Date::new(&JsValue::from_f64(ts));
    let opts = to_js(&json!({"day": "numeric", "month": "short"}));
    let loc = if lang() == Lang::Zh { JsValue::from_str("zh-CN") } else { JsValue::UNDEFINED };
    jcall(&d, "toLocaleDateString", &[loc, opts]).ok().and_then(|v| v.as_string()).unwrap_or_default()
}

fn item_html(s: &NbSt, it: &Item, own: bool) -> String {
    let ed = own && s.editing.as_deref() == Some(it.id.as_str());
    let del = s.confirm_del.as_deref() == Some(it.id.as_str());
    let q = if !it.quote.is_empty() {
        let qv = u16(&it.quote);
        let shown = if qv.len() > 260 { format!("{}…", String::from_utf16_lossy(&qv[..260])) } else { it.quote.clone() };
        format!("<blockquote class=\"nbq\" data-tr=\"1\">{}</blockquote>", esc(&shown))
    } else {
        String::new()
    };
    let body = if ed {
        format!(
            "<textarea id=\"nbDraft\" rows=\"4\" data-tr=\"1\" placeholder=\"{}\">{}</textarea><div class=\"nbrow\"><button class=\"btn\" data-a=\"save\">Save</button><button class=\"btn ghost\" data-a=\"cancel\">Cancel</button></div>",
            esc(&lp("Write your note…", "写下你的笔记…")),
            esc(&s.draft)
        )
    } else if !it.note.is_empty() {
        format!("<div class=\"nbn\" data-tr=\"1\">{}</div>", esc(&it.note))
    } else {
        String::new()
    };
    format!(
        "<div class=\"nbi\" data-id=\"{id}\" style=\"--c:{c}\">\n   {t}\n   {q}\n   {body}\n   <div class=\"nbrow nbm\">\n    {go}\n    {edit}\n    {colors}\n    {vis}\n    {delb}\n    <span class=\"nbd\">{date}</span>\n   </div></div>",
        id = it.id,
        c = hlc(&it.color),
        t = if s.scope == "all" { format!("<div class=\"nbt\">{}</div>", esc(&topic_label(&it.topic))) } else { String::new() },
        q = q,
        body = body,
        go = if !it.quote.is_empty() { "<button class=\"lnk\" data-a=\"go\">Go to</button>" } else { "" },
        edit = if own && !ed { format!("<button class=\"lnk\" data-a=\"edit\">{}</button>", if !it.note.is_empty() { "Edit note" } else { "Add note" }) } else { String::new() },
        colors = if own {
            HLC.iter().map(|(c, v)| format!("<button class=\"dot8 {}\" data-a=\"color\" data-c=\"{}\" style=\"--c:{}\" aria-label=\"Colour {}\"></button>", if it.color == *c { "on" } else { "" }, c, v, c)).collect::<String>()
        } else {
            String::new()
        },
        vis = if own && s.mode == "db" { format!("<button class=\"lnk vis\" data-a=\"vis\">{}</button>", if it.vis == "shared" { "Shared" } else { "Private" }) } else { String::new() },
        delb = if own { format!("<button class=\"lnk danger\" data-a=\"del\">{}</button>", if del { "Click again to delete" } else { "Delete" }) } else { String::new() },
        date = fmt_date(it.ts)
    )
}

fn patch_names() {
    let ps = with(|s| s.nb.prof.clone());
    let zh = lang() == Lang::Zh;
    let nb = nb_el();
    for bx in qsa(&nb, "[data-uid-box]") {
        let p = ps.get(&attr(&bx, "data-uid-box")).cloned().unwrap_or(Value::Null);
        if let Some(n) = qs(&bx, ".nbname") {
            let name = p.get("name").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| if zh { "某同学".into() } else { "Someone".into() });
            set_text(&n, &name);
        }
        if let (Some(im), Some(u)) = (qs(&bx, "img"), p.get("avatarUrl").and_then(|x| x.as_str()).filter(|s| !s.is_empty())) {
            set_attr(&im, "src", u);
            set_hidden(&im, false);
        }
    }
    let uid = with(|s| s.nb.uid.clone());
    if let (Some(me_n), Some(uid)) = (qs(&nb, ".nbbox.mine small"), uid) {
        if let Some(n) = ps.get(&uid).and_then(|p| p.get("name")).and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
            set_text(&me_n, n);
        }
    }
}

pub fn render() {
    fab();
    let k = key_now();
    let open = with(|s| s.nb.open);
    let nb = nb_el();
    let body = doc().body().unwrap();
    if !open || k.is_none() {
        set_hidden(&nb, true);
        rm_class(&body, "nb-open");
        return;
    }
    let k = k.unwrap();
    set_hidden(&nb, false);
    add_class(&body, "nb-open");
    // profiles of the people whose notebooks are shown
    let need: Vec<String> = with(|s| {
        let mut ids: Vec<String> = s.nb.uid.iter().cloned().collect();
        for (id, m) in &s.nb.others {
            if m.values().any(|it| s.nb.scope == "all" || it.topic == k) {
                ids.push(id.clone());
            }
        }
        ids.into_iter().filter(|id| !s.nb.prof.contains_key(id)).collect()
    });
    let user = USER.with(|u| u.borrow().clone());
    if !need.is_empty() && user.is_some() && !with(|s| s.nb.prof_busy) {
        with(|s| s.nb.prof_busy = true);
        let u = user.unwrap();
        spawn_local(async move {
            let arr = Array::new();
            for id in &need {
                arr.push(&JsValue::from_str(id));
            }
            match jcall(&u, "profiles", &[arr.into()]) {
                Ok(p) => match jawait(p).await {
                    Ok(r) => {
                        let v = to_value(&r);
                        with(|s| {
                            if let Some(o) = v.as_object() {
                                for (a, b) in o {
                                    s.nb.prof.insert(a.clone(), b.clone());
                                }
                            }
                            s.nb.prof_busy = false;
                        });
                        if with(|s| s.nb.open) {
                            patch_names();
                        }
                    }
                    Err(_) => with(|s| s.nb.prof_busy = false),
                },
                Err(_) => with(|s| s.nb.prof_busy = false),
            }
        });
    }
    let focus_draft = doc().active_element().map(|e| e.id() == "nbDraft").unwrap_or(false);
    let h = with(|s| {
        let in_scope = |it: &Item| s.nb.scope == "all" || it.topic == k;
        let mut mine: Vec<Item> = s.nb.mine().into_values().filter(|i| in_scope(i)).collect();
        mine.sort_by(|a, b| {
            if a.topic == b.topic {
                let d = a.sec - b.sec;
                let d = if d != 0.0 { d } else { a.ts - b.ts };
                d.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                // String(a).localeCompare(b)
                let r = js_sys::JsString::from(a.topic.as_str()).locale_compare(&b.topic, &Array::new(), &js_sys::Object::new());
                r.cmp(&0)
            }
        });
        let others: Vec<(String, Vec<Item>)> = s
            .nb
            .others
            .iter()
            .map(|(id, m)| {
                let mut l: Vec<Item> = m.values().filter(|i| in_scope(i)).cloned().collect();
                l.sort_by(|a, b| {
                    let d = a.sec - b.sec;
                    let d = if d != 0.0 { d } else { a.ts - b.ts };
                    d.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
                });
                (id.clone(), l)
            })
            .filter(|(_, l)| !l.is_empty())
            .collect();
        let me = &s.nb.me;
        let my_name = s
            .nb
            .uid
            .as_ref()
            .and_then(|u| s.nb.prof.get(u))
            .and_then(|p| p.get("name"))
            .and_then(|x| x.as_str())
            .filter(|x| !x.is_empty())
            .or_else(|| me.get("name").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string();
        let status = match s.nb.mode {
            "local" => "<p class=\"nbstat\">Your notes are saved in this browser only. Open the page in Claude while signed in to keep them in your own notebook and share them with classmates.</p>",
            "local-ro" => "<p class=\"nbstat\">You can read classmates' shared notes. Your own notes are kept in this browser until the owner gives you Contributor access.</p>",
            _ => "",
        };
        let avatar = me.get("avatarUrl").and_then(|x| x.as_str()).unwrap_or("");
        let mut h = format!(
            "<div class=\"nbhead\"><h3>Notebook</h3><button class=\"chipbtn\" data-a=\"close\" aria-label=\"Close\">✕</button></div>\n   <div class=\"seg nbscope\"><button data-a=\"scope\" data-s=\"topic\" aria-pressed=\"{}\">This page</button><button data-a=\"scope\" data-s=\"all\" aria-pressed=\"{}\">All topics</button></div>\n   {}\n   <section class=\"nbbox mine\" style=\"--u:{}\">\n    <div class=\"nbh\"><img src=\"{}\" alt=\"\" {}><div class=\"nbwho\"><b>My notebook</b><small data-tr=\"1\">{}</small></div><span class=\"nbc\">{}</span></div>\n    {}\n    <p class=\"nbhint\">Select any text in the notes or formula sheet to highlight it.</p>\n    <button class=\"btn ghost nbadd\" data-a=\"add\">＋ Add a note</button>\n    <div class=\"nbl\">{}</div>\n   </section>\n   <div class=\"nbhead2\"><h4>Classmates' notebooks</h4><label class=\"nbchk\"><input type=\"checkbox\" id=\"nbShow\" {}> <span>Show their highlights in the text</span></label></div>\n   ",
            s.nb.scope == "topic",
            s.nb.scope == "all",
            status,
            me.get("color").and_then(|x| x.as_str()).unwrap_or("var(--accent)"),
            esc(avatar),
            if avatar.is_empty() { "hidden" } else { "" },
            esc(&my_name),
            mine.len(),
            if s.nb.mode == "db" {
                format!(
                    "<div class=\"nbdef\"><span>New highlights are</span><div class=\"seg\"><button data-a=\"def\" data-v=\"private\" aria-pressed=\"{}\">Private</button><button data-a=\"def\" data-v=\"shared\" aria-pressed=\"{}\">Shared</button></div></div><p class=\"nbhint\">Private notes are visible only to you. Shared notes appear in your container for everyone who opens this page.</p>",
                    s.nb.def_vis == "private",
                    s.nb.def_vis == "shared"
                )
            } else {
                String::new()
            },
            if mine.is_empty() { "<p class=\"nbempty\">No highlights or notes here yet.</p>".to_string() } else { mine.iter().map(|it| item_html(&s.nb, it, true)).collect() },
            if s.nb.show_others { "checked" } else { "" }
        );
        if others.is_empty() {
            h.push_str("<p class=\"nbempty\">No classmates have shared notes here yet.</p>");
        } else {
            for (id, l) in &others {
                let p = s.nb.prof.get(id).cloned().unwrap_or(Value::Null);
                let op = s.nb.open_box.get(id).copied();
                let is_open = match op {
                    None => l.len() <= 6,
                    Some(o) => o,
                };
                let av = p.get("avatarUrl").and_then(|x| x.as_str()).unwrap_or("");
                h.push_str(&format!(
                    "<details class=\"nbbox other\" data-uid-box=\"{}\" style=\"--u:{}\" {}>\n     <summary><span class=\"nbh\"><img src=\"{}\" alt=\"\" {}><span class=\"nbwho\"><b class=\"nbname\" data-tr=\"1\"></b></span><span class=\"nbc\">{}</span></span></summary>\n     <div class=\"nbl\">{}</div></details>",
                    esc(id),
                    p.get("color").and_then(|x| x.as_str()).unwrap_or("var(--muted)"),
                    if is_open { "open" } else { "" },
                    esc(av),
                    if av.is_empty() { "hidden" } else { "" },
                    l.len(),
                    l.iter().map(|it| item_html(&s.nb, it, false)).collect::<String>()
                ));
            }
        }
        h
    });
    set_html(&nb, &h);
    patch_names();
    i18n::translate(&nb);
    if let Some(ta) = by_id("nbDraft") {
        let fnext = with(|s| s.nb.focus_next);
        if focus_draft || fnext {
            let _ = hel(&ta).focus();
            if let Some(t) = ta.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                let n = t.value().encode_utf16().count() as u32;
                let _ = t.set_selection_start(Some(n));
                let _ = t.set_selection_end(Some(n));
            }
            with(|s| s.nb.focus_next = false);
        }
    }
}

fn find_item(id: &str) -> Option<(Item, bool)> {
    with(|s| {
        if let Some(it) = s.nb.mine().get(id) {
            return Some((it.clone(), true));
        }
        for m in s.nb.others.values() {
            if let Some(it) = m.get(id) {
                return Some((it.clone(), false));
            }
        }
        None
    })
}

fn nb_click(t: &Element) {
    let b = match closest(t, "[data-a]") {
        Some(b) => b,
        None => return,
    };
    let a = attr(&b, "data-a");
    match a.as_str() {
        "close" => {
            with(|s| s.nb.open = false);
            render();
            return;
        }
        "scope" => {
            let v = attr(&b, "data-s");
            with(|s| s.nb.scope = v);
            render();
            return;
        }
        "def" => {
            let v = attr(&b, "data-v");
            store_set("nbDefVis", &Value::String(v.clone()));
            with(|s| s.nb.def_vis = v);
            render();
            return;
        }
        "add" => {
            let k = key_now().unwrap_or_default();
            let it = Item { id: new_id(), topic: k, sec: -1.0, quote: String::new(), occ: 0.0, color: "yellow".into(), note: String::new(), ts: date_now(), vis: "private".into(), local: false, uid: None };
            let vis = with(|s| {
                s.nb.editing = Some(it.id.clone());
                s.nb.draft.clear();
                s.nb.focus_next = true;
                if s.nb.mode == "db" {
                    s.nb.def_vis.clone()
                } else {
                    "private".into()
                }
            });
            let mut it2 = it.clone();
            it2.vis = vis.clone();
            optimistic(&it2, &vis);
            write(it, vis);
            return;
        }
        _ => {}
    }
    let card = match closest(&b, ".nbi") {
        Some(c) => c,
        None => return,
    };
    let id = attr(&card, "data-id");
    let (it, own) = match find_item(&id) {
        Some(x) => x,
        None => return,
    };
    if a == "go" {
        go_to(it);
        return;
    }
    if !own {
        return;
    }
    match a.as_str() {
        "edit" => {
            with(|s| {
                s.nb.editing = Some(id.clone());
                s.nb.draft = it.note.clone();
                s.nb.focus_next = true;
            });
            render();
        }
        "cancel" => {
            with(|s| s.nb.editing = None);
            render();
        }
        "save" => {
            let note = by_id("nbDraft").map(|e| input_value(&e)).unwrap_or_default();
            with(|s| s.nb.editing = None);
            let mut upd = it.clone();
            upd.note = js_trim(&note).to_string();
            if upd.quote.is_empty() && upd.note.is_empty() {
                delete(it);
                return;
            }
            with(|s| {
                if s.nb.mode == "db" {
                    let m = if it.vis == "shared" { &mut s.nb.shar } else { &mut s.nb.prv };
                    m.insert(id.clone(), upd.clone());
                }
            });
            changed();
            let vis = it.vis.clone();
            write(upd, vis);
        }
        "color" => {
            let mut upd = it.clone();
            upd.color = attr(&b, "data-c");
            with(|s| {
                if s.nb.mode == "db" {
                    let m = if it.vis == "shared" { &mut s.nb.shar } else { &mut s.nb.prv };
                    m.insert(id.clone(), upd.clone());
                }
            });
            changed();
            let vis = it.vis.clone();
            write(upd, vis);
        }
        "vis" => {
            let to = if it.vis == "shared" { "private" } else { "shared" };
            mv(it, to.into());
        }
        "del" => {
            if with(|s| s.nb.confirm_del.as_deref() != Some(id.as_str())) {
                with(|s| s.nb.confirm_del = Some(id.clone()));
                render();
                let id2 = id.clone();
                timeout(3000, move || {
                    if with(|s| s.nb.confirm_del.as_deref() == Some(id2.as_str())) {
                        with(|s| s.nb.confirm_del = None);
                        render();
                    }
                });
                return;
            }
            with(|s| {
                s.nb.confirm_del = None;
                if s.nb.mode == "db" {
                    s.nb.prv.shift_remove(&id);
                    s.nb.shar.shift_remove(&id);
                }
            });
            changed();
            delete(it);
        }
        _ => {}
    }
}

fn go_to(it: Item) {
    let k = key_now();
    let it2 = it.clone();
    let find = move || {
        if let Some(m) = qs(&app(), &format!("mark.hl[data-hid=\"{}\"]", it2.id)) {
            if let Some(d) = closest(&m, "details") {
                set_details_open(&d, true);
            }
            let o = web_sys::ScrollIntoViewOptions::new();
            o.set_block(web_sys::ScrollLogicalPosition::Center);
            o.set_behavior(web_sys::ScrollBehavior::Smooth);
            m.scroll_into_view_with_scroll_into_view_options(&o);
            add_class(&m, "pulse");
            timeout(1800, move || rm_class(&m, "pulse"));
        } else {
            toast(&lp("This highlight is in the other language view. Switch language to see it in the text.", "此高亮位于另一种语言视图中，切换语言即可在正文中看到。"));
        }
    };
    if Some(&it.topic) == k.as_ref() {
        if it.uid.is_some() && !with(|s| s.nb.show_others) {
            with(|s| s.nb.show_others = true);
            store_set("nbShowOthers", &Value::Bool(true));
            apply_marks();
            render();
        }
        find();
        return;
    }
    if it.topic == "fx" {
        with(|s| {
            s.fx = formulas::FxSt::default();
        });
        go("formulas", GoOpts::default());
    } else {
        set_topic(&it.topic, true);
        go("learn", GoOpts::default());
    }
    timeout(120, find);
}

pub fn on_mark_click(t: &Element) -> bool {
    let m = match closest(t, "#app mark.hl") {
        Some(m) => m,
        None => return false,
    };
    let selected = win().get_selection().ok().flatten().and_then(|s| s.to_string().as_string()).unwrap_or_default();
    if !selected.is_empty() {
        return false;
    }
    with(|s| s.nb.open = true);
    let id = attr(&m, "data-hid");
    render();
    raf(move || {
        if let Some(c) = qs(&nb_el(), &format!(".nbi[data-id=\"{}\"]", id)) {
            if let Some(d) = closest(&c, "details") {
                set_details_open(&d, true);
            }
            let o = web_sys::ScrollIntoViewOptions::new();
            o.set_block(web_sys::ScrollLogicalPosition::Center);
            c.scroll_into_view_with_scroll_into_view_options(&o);
            add_class(&c, "pulse");
            timeout(1600, move || rm_class(&c, "pulse"));
        }
    });
    true
}

fn toast(msg: &str) {
    if let Some(t) = by_id("nbToast") {
        set_text(&t, msg);
        set_hidden(&t, false);
        let old = with(|s| s.nb.toast_t);
        clear_timeout(old);
        let id = timeout(3500, move || set_hidden(&t, true));
        with(|s| s.nb.toast_t = id);
    }
}

/// Create the toolbar, drawer, button and toast, and wire their events.
pub fn install() {
    let d = doc();
    let body = d.body().unwrap();
    let tbe = d.create_element("div").unwrap();
    tbe.set_id("nbTb");
    tbe.set_class_name("nbtb");
    set_hidden(&tbe, true);
    set_attr(&tbe, "role", "toolbar");
    let _ = body.append_child(&tbe);
    tb_render();
    let md = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| e.prevent_default());
    let _ = tbe.add_event_listener_with_callback("mousedown", md.as_ref().unchecked_ref());
    md.forget();
    let tc = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            tb_click(&t);
        }
    });
    let _ = tbe.add_event_listener_with_callback("click", tc.as_ref().unchecked_ref());
    tc.forget();

    let nb = d.create_element("aside").unwrap();
    nb.set_id("nb");
    nb.set_class_name("nb");
    set_hidden(&nb, true);
    set_attr(&nb, "aria-label", "Notebook");
    let _ = body.append_child(&nb);
    let nc = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            nb_click(&t);
        }
    });
    let _ = nb.add_event_listener_with_callback("click", nc.as_ref().unchecked_ref());
    nc.forget();
    let ch = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            if t.id() == "nbShow" {
                let v = t.dyn_ref::<web_sys::HtmlInputElement>().map(|i| i.checked()).unwrap_or(false);
                with(|s| s.nb.show_others = v);
                store_set("nbShowOthers", &Value::Bool(v));
                apply_marks();
            }
        }
    });
    let _ = nb.add_event_listener_with_callback("change", ch.as_ref().unchecked_ref());
    ch.forget();
    let inp = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            if t.id() == "nbDraft" {
                let v = input_value(&t);
                with(|s| s.nb.draft = v);
            }
        }
    });
    let _ = nb.add_event_listener_with_callback("input", inp.as_ref().unchecked_ref());
    inp.forget();
    let tg = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            if t.matches("details[data-uid-box]").unwrap_or(false) {
                let id = attr(&t, "data-uid-box");
                let o = details_open(&t);
                with(|s| s.nb.open_box.insert(id, o));
            }
        }
    });
    let _ = nb.add_event_listener_with_callback_and_bool("toggle", tg.as_ref().unchecked_ref(), true);
    tg.forget();

    let fb = d.create_element("button").unwrap();
    fb.set_id("nbFab");
    fb.set_class_name("nbfab");
    set_hidden(&fb, true);
    let _ = body.append_child(&fb);
    let fc = Closure::<dyn FnMut()>::new(|| {
        with(|s| s.nb.open = !s.nb.open);
        render();
    });
    let _ = fb.add_event_listener_with_callback("click", fc.as_ref().unchecked_ref());
    fc.forget();

    let te = d.create_element("div").unwrap();
    te.set_id("nbToast");
    te.set_class_name("nbtoast");
    set_hidden(&te, true);
    let _ = body.append_child(&te);

    let sc = Closure::<dyn FnMut()>::new(|| {
        let t = with(|s| s.nb.sel_t);
        clear_timeout(t);
        let id = timeout(220, on_selection);
        with(|s| s.nb.sel_t = id);
    });
    let _ = d.add_event_listener_with_callback("selectionchange", sc.as_ref().unchecked_ref());
    sc.forget();
}
