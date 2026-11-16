//! Scroll memory per page/topic, header metrics, section jump, reading progress, back-to-top.
use crate::util::*;
use crate::*;
use serde_json::{Map, Value};
use std::cell::Cell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy)]
pub struct Anchor {
    pub i: usize,
    pub off: f64,
    pub f: Option<f64>,
}

pub struct NavSt {
    pub k: Option<String>,
    pub lock: bool,
    pub going: bool,
    pub pos: Map<String, Value>,
    pub anc: Map<String, Value>,
    pub run: u32,
    t: i32,
    t2: i32,
}
impl NavSt {
    pub fn load() -> NavSt {
        let rd = |k: &str| ls_get(k).and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default();
        let _ = win().history().map(|h| h.set_scroll_restoration(web_sys::ScrollRestoration::Manual));
        NavSt { k: None, lock: false, going: false, pos: rd("sh_scroll"), anc: rd("sh_anchor"), run: 0, t: 0, t2: 0 }
    }
}

thread_local! {
    static HDR_H: Cell<f64> = Cell::new(110.0);
    static SCROLL_TICK: Cell<bool> = Cell::new(false);
    static PROG_TICK: Cell<bool> = Cell::new(false);
}
pub fn hdr_h() -> f64 {
    HDR_H.with(|h| h.get())
}

pub fn key() -> String {
    with(|s| if s.tab == "learn" { format!("learn:{}", s.topic) } else { s.tab.clone() })
}
pub fn bump_run() -> u32 {
    with(|s| {
        s.nav.run += 1;
        s.nav.run
    })
}

fn persist(now: bool) {
    let write_pos = || {
        let p = with(|s| Value::Object(s.nav.pos.clone()).to_string());
        ls_set("sh_scroll", &p);
    };
    let write_anc = || {
        let p = with(|s| Value::Object(s.nav.anc.clone()).to_string());
        ls_set("sh_anchor", &p);
    };
    let (t, t2) = with(|s| (s.nav.t, s.nav.t2));
    clear_timeout(t);
    if now {
        write_pos();
        write_anc();
    } else {
        let id = timeout(250, write_pos);
        clear_timeout(t2);
        let id2 = timeout(300, write_anc);
        with(|s| {
            s.nav.t = id;
            s.nav.t2 = id2;
        });
    }
}

pub fn anchor() -> Option<Anchor> {
    if cur_tab() != "learn" {
        return None;
    }
    let hh = hdr_h();
    for (i, s) in qsa(&app(), ".reader>section").iter().enumerate() {
        let r = s.get_bounding_client_rect();
        if r.bottom() > hh + 1.0 {
            let f = if r.height() > 0.0 { parse_float(&to_fixed((hh - r.top()) / r.height(), 4)) } else { 0.0 };
            return Some(Anchor { i, off: (hh - r.top() + 0.5).floor(), f: Some(f) });
        }
    }
    None
}
fn anchor_json(a: &Anchor) -> Value {
    serde_json::json!({"i": a.i, "off": a.off, "f": a.f})
}
fn anchor_from(v: &Value) -> Option<Anchor> {
    Some(Anchor { i: v.get("i")?.as_u64()? as usize, off: v.get("off").and_then(|x| x.as_f64()).unwrap_or(0.0), f: v.get("f").and_then(|x| x.as_f64()) })
}

fn save() {
    let k = match with(|s| s.nav.k.clone()) {
        Some(k) => k,
        None => return,
    };
    let y = scroll_y().round();
    with(|s| {
        s.nav.pos.insert(k.clone(), Value::from(y as i64));
    });
    if cur_tab() == "learn" && k == key() {
        let a = anchor();
        with(|s| match a {
            Some(a) => {
                s.nav.anc.insert(k.clone(), anchor_json(&a));
            }
            None => {
                s.nav.anc.shift_remove(&k);
            }
        });
    }
}

pub fn leave() {
    let (k, lock) = with(|s| (s.nav.k.is_some(), s.nav.lock));
    if k && !lock {
        save();
        persist(false);
    }
    with(|s| s.nav.lock = true);
}

pub fn to_anchor(a: Anchor, run: u32, frac: bool) {
    let t0 = now();
    step_anchor(a, run, frac, t0, -9.0, 0);
}
fn step_anchor(a: Anchor, run: u32, frac: bool, t0: f64, last: f64, stable: u32) {
    if run != with(|s| s.nav.run) {
        return;
    }
    let secs = qsa(&app(), ".reader>section");
    let s = match secs.get(a.i) {
        Some(s) => s.clone(),
        None => {
            with(|s| s.nav.lock = false);
            return;
        }
    };
    let sr = s.get_bounding_client_rect();
    let add = if frac && a.f.is_some() { a.f.unwrap() * sr.height() } else { a.off };
    let target = (sr.top() + scroll_y() - hdr_h() + add + 0.5).floor().max(0.0);
    scroll_to(target);
    let y = scroll_y();
    let stable = if (y - last).abs() < 2.0 { stable + 1 } else { 0 };
    if stable >= 2 || now() - t0 > 1500.0 {
        with(|s| s.nav.lock = false);
        return;
    }
    raf(move || step_anchor(a, run, frac, t0, y, stable));
}

pub fn arrive(keep: bool) {
    let k = key();
    let cur = with(|s| s.nav.k.clone());
    if keep && Some(&k) == cur.as_ref() {
        let a = if cur_tab() == "learn" && scroll_y() > 0.0 { with(|s| s.nav.anc.get(&k).and_then(anchor_from)) } else { None };
        if let Some(a) = a {
            with(|s| s.nav.lock = true);
            let run = bump_run();
            to_anchor(a, run, true);
        } else {
            with(|s| s.nav.lock = false);
        }
        return;
    }
    with(|s| s.nav.k = Some(k.clone()));
    let y = with(|s| s.nav.pos.get(&k).and_then(|v| v.as_f64()).unwrap_or(0.0));
    let run = bump_run();
    if y == 0.0 {
        scroll_to(0.0);
        with(|s| s.nav.lock = false);
        return;
    }
    if cur_tab() == "learn" {
        if let Some(a) = with(|s| s.nav.anc.get(&k).and_then(anchor_from)) {
            to_anchor(a, run, false);
            return;
        }
    }
    let t0 = now();
    step_y(y, run, t0);
}
fn step_y(y: f64, run: u32, t0: f64) {
    if run != with(|s| s.nav.run) {
        return;
    }
    let de = doc().document_element().unwrap();
    let max = de.scroll_height() as f64 - win().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    scroll_to(y.min(max));
    if max >= y - 2.0 || now() - t0 > 2000.0 {
        with(|s| s.nav.lock = false);
        return;
    }
    raf(move || step_y(y, run, t0));
}

pub fn stop() {
    with(|s| {
        if s.nav.lock && !s.nav.going {
            s.nav.run += 1;
            s.nav.lock = false;
        }
    });
}

pub fn sec_go(i: usize) {
    // jumping to an "extra" section while notes-only is on first turns notes-only off
    if let Some(s) = qsa(&app(), ".reader>section").get(i) {
        if has_class(s, "xtra") && with(|s| s.src_only) {
            if by_id("srcTog").is_some() {
                learn::toggle_src_only();
            }
        }
    }
    let s = match qsa(&app(), ".reader>section").get(i) {
        Some(s) => s.clone(),
        None => return,
    };
    with(|st| {
        st.nav.run += 1;
        st.nav.lock = false;
    });
    let s2 = s.clone();
    let j = move || {
        let t = (s2.get_bounding_client_rect().top() + scroll_y() - hdr_h() - 10.0 + 0.5).floor().max(0.0);
        scroll_to(t);
    };
    j();
    let j1 = j.clone();
    raf(move || {
        j1();
        raf(j);
    });
    add_class(&s, "secflash");
    timeout(1400, move || rm_class(&s, "secflash"));
}

pub fn header_metrics() {
    let set = || {
        if let Some(h) = qs(&doc().document_element().unwrap(), "header.top") {
            let v = (h.get_bounding_client_rect().bottom() + 0.5).floor().max(0.0);
            HDR_H.with(|x| x.set(v));
            let de: web_sys::HtmlElement = doc().document_element().unwrap().unchecked_into();
            let _ = de.style().set_property("--hdr", &format!("{}px", num_str(v)));
        }
    };
    set();
}

pub fn install() {
    let d = doc();
    // reading progress bar + back-to-top button
    if let Some(h) = qs(&d.document_element().unwrap(), "header.top") {
        let p = d.create_element("div").unwrap();
        p.set_id("prog");
        let _ = h.append_child(&p);
        let ro_cb = Closure::<dyn FnMut()>::new(header_metrics);
        if let Ok(ro) = web_sys::ResizeObserver::new(ro_cb.as_ref().unchecked_ref()) {
            ro.observe(&h);
            std::mem::forget(ro);
        }
        ro_cb.forget();
    }
    let tb = d.create_element("button").unwrap();
    tb.set_id("toTop");
    set_attr(&tb, "type", "button");
    set_hidden(&tb, true);
    set_attr(&tb, "aria-label", "Back to top");
    set_text(&tb, "↑");
    let _ = d.body().unwrap().append_child(&tb);

    let w = win();
    let rs = Closure::<dyn FnMut()>::new(header_metrics);
    let _ = w.add_event_listener_with_callback("resize", rs.as_ref().unchecked_ref());
    rs.forget();
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_passive(true);
    for ev in ["wheel", "touchstart", "keydown", "mousedown"] {
        let cb = Closure::<dyn FnMut()>::new(stop);
        let _ = w.add_event_listener_with_callback_and_add_event_listener_options(ev, cb.as_ref().unchecked_ref(), &opts);
        cb.forget();
    }
    let sc = Closure::<dyn FnMut()>::new(|| {
        if !SCROLL_TICK.with(|t| t.replace(true)) {
            raf(|| {
                SCROLL_TICK.with(|t| t.set(false));
                let ok = with(|s| !s.nav.lock && s.nav.k.is_some() && !s.nav.going);
                if ok {
                    save();
                    persist(false);
                }
            });
        }
        if !PROG_TICK.with(|t| t.replace(true)) {
            raf(progress);
        }
    });
    let _ = w.add_event_listener_with_callback_and_add_event_listener_options("scroll", sc.as_ref().unchecked_ref(), &opts);
    sc.forget();
    let ph = Closure::<dyn FnMut()>::new(|| {
        let ok = with(|s| !s.nav.lock && s.nav.k.is_some());
        if ok {
            save();
        }
        persist(true);
    });
    let _ = w.add_event_listener_with_callback("pagehide", ph.as_ref().unchecked_ref());
    ph.forget();
    let vc = Closure::<dyn FnMut()>::new(|| {
        if doc().visibility_state() == web_sys::VisibilityState::Hidden {
            persist(true);
        }
    });
    let _ = d.add_event_listener_with_callback("visibilitychange", vc.as_ref().unchecked_ref());
    vc.forget();
}

fn progress() {
    PROG_TICK.with(|t| t.set(false));
    let y = scroll_y();
    let de = doc().document_element().unwrap();
    let max = de.scroll_height() as f64 - win().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    if let Some(p) = by_id("prog") {
        let v = if cur_tab() == "learn" && max > 40.0 { (y / max).min(1.0) } else { 0.0 };
        let _ = hel(&p).style().set_property("transform", &format!("scaleX({})", num_str(v)));
    }
    if let Some(t) = by_id("toTop") {
        set_hidden(&t, y < 700.0);
    }
}

pub fn to_top() {
    with(|s| {
        s.nav.run += 1;
        s.nav.lock = false;
    });
    let reduce = win().match_media("(prefers-reduced-motion: reduce)").ok().flatten().map(|m| m.matches()).unwrap_or(false);
    let o = web_sys::ScrollToOptions::new();
    o.set_top(0.0);
    o.set_behavior(if reduce { web_sys::ScrollBehavior::Auto } else { web_sys::ScrollBehavior::Smooth });
    win().scroll_to_with_scroll_to_options(&o);
}
