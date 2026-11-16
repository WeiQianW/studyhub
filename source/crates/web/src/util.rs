//! DOM, storage and number-formatting helpers.
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, Window};

pub fn win() -> Window {
    web_sys::window().unwrap()
}
pub fn doc() -> Document {
    win().document().unwrap()
}
pub fn by_id(id: &str) -> Option<Element> {
    doc().get_element_by_id(id)
}
pub fn app() -> Element {
    by_id("app").unwrap()
}
pub fn qs(root: &Element, sel: &str) -> Option<Element> {
    root.query_selector(sel).ok().flatten()
}
pub fn qsa(root: &Element, sel: &str) -> Vec<Element> {
    let mut out = vec![];
    if let Ok(l) = root.query_selector_all(sel) {
        for i in 0..l.length() {
            if let Some(n) = l.item(i) {
                if let Ok(e) = n.dyn_into::<Element>() {
                    out.push(e);
                }
            }
        }
    }
    out
}
pub fn dqsa(sel: &str) -> Vec<Element> {
    qsa(&doc().document_element().unwrap(), sel)
}
pub fn closest(e: &Element, sel: &str) -> Option<Element> {
    e.closest(sel).ok().flatten()
}
pub fn attr(e: &Element, a: &str) -> String {
    e.get_attribute(a).unwrap_or_default()
}
pub fn set_attr(e: &Element, a: &str, v: &str) {
    let _ = e.set_attribute(a, v);
}
pub fn set_html(e: &Element, h: &str) {
    e.set_inner_html(h);
}
pub fn set_text(e: &Element, t: &str) {
    e.set_text_content(Some(t));
}
pub fn toggle_class(e: &Element, c: &str, on: bool) {
    let _ = e.class_list().toggle_with_force(c, on);
}
pub fn add_class(e: &Element, c: &str) {
    let _ = e.class_list().add_1(c);
}
pub fn rm_class(e: &Element, c: &str) {
    let _ = e.class_list().remove_1(c);
}
pub fn has_class(e: &Element, c: &str) -> bool {
    e.class_list().contains(c)
}
pub fn hel(e: &Element) -> HtmlElement {
    e.clone().unchecked_into()
}
pub fn set_hidden(e: &Element, h: bool) {
    hel(e).set_hidden(h);
}
pub fn input_value(e: &Element) -> String {
    if let Some(i) = e.dyn_ref::<web_sys::HtmlInputElement>() {
        return i.value();
    }
    if let Some(s) = e.dyn_ref::<web_sys::HtmlSelectElement>() {
        return s.value();
    }
    if let Some(t) = e.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        return t.value();
    }
    String::new()
}
pub fn set_input_value(e: &Element, v: &str) {
    if let Some(i) = e.dyn_ref::<web_sys::HtmlInputElement>() {
        i.set_value(v);
    } else if let Some(t) = e.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        t.set_value(v);
    } else if let Some(s) = e.dyn_ref::<web_sys::HtmlSelectElement>() {
        s.set_value(v);
    }
}
pub fn dataset(e: &Element, k: &str) -> Option<String> {
    e.get_attribute(&format!("data-{}", k))
}
pub fn details_open(e: &Element) -> bool {
    e.has_attribute("open")
}
pub fn set_details_open(e: &Element, o: bool) {
    if let Some(d) = e.dyn_ref::<web_sys::HtmlDetailsElement>() {
        d.set_open(o);
    }
}
pub fn scroll_y() -> f64 {
    win().scroll_y().unwrap_or(0.0)
}
pub fn scroll_to(y: f64) {
    win().scroll_to_with_x_and_y(0.0, y);
}
pub fn now() -> f64 {
    win().performance().map(|p| p.now()).unwrap_or(0.0)
}
pub fn date_now() -> f64 {
    js_sys::Date::now()
}
pub fn random() -> f64 {
    js_sys::Math::random()
}

/// setTimeout with a one-shot Rust closure.
pub fn timeout<F: FnOnce() + 'static>(ms: i32, f: F) -> i32 {
    let cb = Closure::once_into_js(f);
    win().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms).unwrap_or(0)
}
pub fn clear_timeout(id: i32) {
    win().clear_timeout_with_handle(id);
}
pub fn raf<F: FnOnce() + 'static>(f: F) -> i32 {
    let cb = Closure::once_into_js(f);
    win().request_animation_frame(cb.unchecked_ref()).unwrap_or(0)
}

// ---------- localStorage ----------
pub fn ls() -> Option<web_sys::Storage> {
    win().local_storage().ok().flatten()
}
pub fn ls_get(k: &str) -> Option<String> {
    ls()?.get_item(k).ok().flatten()
}
pub fn ls_set(k: &str, v: &str) {
    if let Some(s) = ls() {
        let _ = s.set_item(k, v);
    }
}
/// The original site's `store` (prefix dpsh_, JSON values) — same keys, so saved progress carries over.
pub fn store_get(k: &str) -> Option<serde_json::Value> {
    let v = ls_get(&format!("dpsh_{}", k))?;
    if v.is_empty() {
        return None;
    }
    serde_json::from_str(&v).ok()
}
pub fn store_set(k: &str, v: &serde_json::Value) {
    ls_set(&format!("dpsh_{}", k), &v.to_string());
}

// ---------- JS-compatible number formatting ----------
pub fn to_fixed(v: f64, d: u8) -> String {
    js_sys::Number::from(v).to_fixed(d).map(|s| String::from(s)).unwrap_or_default()
}
/// String(number) exactly as JavaScript prints it.
pub fn num_str(v: f64) -> String {
    js_sys::Number::from(v).to_string(10).map(|s| String::from(s)).unwrap_or_default()
}
pub fn parse_float(s: &str) -> f64 {
    js_sys::Number::parse_float(s)
}
pub fn parse_int(s: &str) -> f64 {
    js_sys::Number::parse_int(s, 10)
}
/// Number(s)
pub fn js_number(s: &str) -> f64 {
    js_sys::Number::new(&JsValue::from_str(s)).value_of()
}
/// The calculators' `fmt(v, d)`: fixed decimals, trailing ".000" removed, 0 decimals at ≥ 10 000.
pub fn fmt(v: f64, d: u8) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    let s = if v.abs() >= 1e4 { to_fixed(v, 0) } else { to_fixed(v, d) };
    // replace(/\.?0+$/, m => m.includes('.') ? '' : m)
    if let Some(dot) = s.find('.') {
        if s[dot + 1..].chars().all(|c| c == '0') {
            return s[..dot].to_string();
        }
    }
    s
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}
pub fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// Remove every `open … close` block (non-greedy), like `replace(/open[\s\S]*?close/g, rep)`.
pub fn strip_blocks(s: &str, open: &str, close: &str, rep: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(open) {
        let after = &rest[i + open.len()..];
        match after.find(close) {
            Some(j) => {
                out.push_str(&rest[..i]);
                out.push_str(rep);
                rest = &after[j + close.len()..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}
/// `replace(/<[^>]+>/g, rep)`
pub fn strip_tags(s: &str, rep: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    let mut last = 0;
    while i < b.len() {
        if b[i] == b'<' {
            if let Some(j) = s[i + 1..].find('>') {
                if j > 0 {
                    out.push_str(&s[last..i]);
                    out.push_str(rep);
                    i = i + 1 + j + 1;
                    last = i;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.push_str(&s[last..]);
    out
}

/// JavaScript `String(value)` for JSON values (arrays join with ",", null → "" inside arrays).
pub fn js_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => num_str(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a.iter().map(js_elem).collect::<Vec<_>>().join(","),
        serde_json::Value::Object(_) => "[object Object]".into(),
    }
}
/// element of Array.prototype.join: null/undefined → ""
pub fn js_elem(v: &serde_json::Value) -> String {
    if v.is_null() {
        String::new()
    } else {
        js_string(v)
    }
}
pub fn js_join(a: &[serde_json::Value], sep: &str) -> String {
    a.iter().map(js_elem).collect::<Vec<_>>().join(sep)
}

pub fn js_log(s: &str) {
    web_sys::console::log_1(&JsValue::from_str(s));
}
