//! Mechatronics Study Hub — the whole study site, written in Rust and compiled to WebAssembly.
//! Content (notes, cards, quiz, formulas, dictionaries) is data in `content.json`;
//! every view, calculator, the PLC simulator, search and the notebook are implemented here.
use std::cell::{Cell, OnceCell, RefCell};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub mod data;
pub mod util;
mod i18n;
mod home;
mod learn;
mod cards;
mod quiz;
mod calc;
mod lab;
mod formulas;
mod extra;
mod search;
mod notebook;
mod nav;
mod events;

use data::Content;
use indexmap::IndexSet;
use util::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
    Both,
}
impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Zh => "zh",
            Lang::Both => "both",
        }
    }
    fn parse(s: &str) -> Lang {
        match s {
            "en" => Lang::En,
            "zh" => Lang::Zh,
            _ => Lang::Both,
        }
    }
}

pub struct St {
    pub tab: String,
    pub topic: String,
    pub done: IndexSet<String>,
    pub known: IndexSet<String>,
    pub best: Option<(f64, f64)>,
    pub src_only: bool,
    pub card: cards::CardSt,
    pub quiz: quiz::QuizSt,
    pub fx: formulas::FxSt,
    pub xs: extra::XSt,
    pub lab: lab::LabSt,
    pub nb: notebook::NbSt,
    pub nav: nav::NavSt,
    pub sx: search::SxSt,
    pub calc_seg: std::collections::HashMap<String, String>,
}

thread_local! {
    static CONTENT: OnceCell<&'static Content> = OnceCell::new();
    static STATE: RefCell<Option<St>> = RefCell::new(None);
    static LANG: Cell<Lang> = Cell::new(Lang::Both);
    pub(crate) static OBSERVER: RefCell<Option<web_sys::MutationObserver>> = RefCell::new(None);
}

pub fn c() -> &'static Content {
    CONTENT.with(|c| *c.get().expect("content"))
}
/// Run `f` with the mutable app state. Keep it short: never call another top-level action inside.
pub fn with<R>(f: impl FnOnce(&mut St) -> R) -> R {
    STATE.with(|s| f(s.borrow_mut().as_mut().expect("state")))
}
pub fn lang() -> Lang {
    LANG.with(|l| l.get())
}
pub fn set_lang(l: Lang) {
    LANG.with(|x| x.set(l));
}

// ---------- language helpers (same rules as the original L / LB / LP) ----------
pub fn tl(en: &str, zh: Option<&str>) -> String {
    match zh {
        None | Some("") => en.to_string(),
        Some(z) => match lang() {
            Lang::En => en.to_string(),
            Lang::Zh => z.to_string(),
            Lang::Both => format!("<span class=\"bi\">{}<span class=\"zhl\">{}</span></span>", en, z),
        },
    }
}
pub fn l(en: &str, zh: &str) -> String {
    tl(en, Some(zh))
}
pub fn tlb(en: &str, zh: Option<&str>) -> String {
    match zh {
        None | Some("") => en.to_string(),
        Some(z) => match lang() {
            Lang::En => en.to_string(),
            Lang::Zh => z.to_string(),
            Lang::Both => {
                let z = strip_blocks(z, "<svg", "</svg>", "");
                let z = strip_formula_divs(&z);
                format!("<div class=\"bi\">{}<div class=\"zhb\">{}</div></div>", en, z)
            }
        },
    }
}
/// `.replace(/<div class="formula"[^>]*>[\s\S]*?<\/div>/g,'')`
fn strip_formula_divs(s: &str) -> String {
    let open = "<div class=\"formula\"";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(open) {
        let after = &rest[i + open.len()..];
        let gt = match after.find('>') {
            Some(g) => g,
            None => break,
        };
        // [^>]* cannot cross a '>' — the first '>' closes the tag
        let body = &after[gt + 1..];
        match body.find("</div>") {
            Some(j) => {
                out.push_str(&rest[..i]);
                rest = &body[j + 6..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}
pub fn tlp(en: &str, zh: Option<&str>) -> String {
    match zh {
        None | Some("") => en.to_string(),
        Some(z) => match lang() {
            Lang::En => en.to_string(),
            Lang::Zh => z.to_string(),
            Lang::Both => format!("{} / {}", en, z),
        },
    }
}
pub fn lp(en: &str, zh: &str) -> String {
    tlp(en, Some(zh))
}
pub fn wk_zh(w: &str) -> String {
    let n: String = w.chars().filter(|c| c.is_ascii_digit()).collect();
    match w.chars().next() {
        Some('W') => format!("第{}周", n),
        Some('T') => format!("主题 {}", n),
        _ => format!("第{}章", n),
    }
}
pub fn wk(t: &data::Topic) -> String {
    if lang() == Lang::Zh {
        wk_zh(&t.wk)
    } else {
        t.wk.clone()
    }
}
pub fn tt(t: &data::Topic) -> String {
    l(&t.title, &t.zh)
}
pub fn ttp(t: &data::Topic) -> String {
    lp(&t.title, &t.zh)
}
pub fn mn(m: &str) -> String {
    let md = &c().mods[m];
    l(&md.name, &md.zh)
}
pub fn zd(k: &str) -> Option<&'static str> {
    c().zd.get(k).map(|s| s.as_str())
}

pub const TABS: [(&str, &str); 8] = [
    ("home", "Overview"),
    ("learn", "Notes"),
    ("cards", "Flashcards"),
    ("quiz", "Quiz"),
    ("calc", "Calculators"),
    ("lab", "PLC Lab"),
    ("formulas", "Formula Sheet"),
    ("extra", "Extra"),
];

// ---------- navigation between tabs ----------
#[derive(Default, Clone, Copy)]
pub struct GoOpts {
    pub keep_scroll: bool,
}
pub fn go(tab: &str, opts: GoOpts) {
    nav::leave();
    with(|s| {
        s.nav.going = true;
        s.tab = tab.to_string();
    });
    lab::stop();
    if let Some(tabs) = by_id("tabs") {
        for b in qsa(&tabs, "button") {
            set_attr(&b, "aria-selected", if attr(&b, "data-tab") == tab { "true" } else { "false" });
        }
    }
    let _ = win().history().map(|h| h.replace_state_with_url(&JsValue::NULL, "", Some(&format!("#{}", tab))));
    match tab {
        "home" => home::render(),
        "learn" => learn::render(),
        "cards" => cards::render(),
        "quiz" => quiz::render(),
        "calc" => calc::render(),
        "lab" => lab::render(),
        "formulas" => formulas::render(),
        _ => extra::render(),
    }
    if tab != "learn" && tab != "formulas" {
        notebook::after_render();
    }
    with(|s| s.nav.going = false);
    nav::arrive(opts.keep_scroll);
}
pub fn cur_tab() -> String {
    with(|s| s.tab.clone())
}
pub fn set_topic(id: &str, persist: bool) {
    with(|s| s.topic = id.to_string());
    if persist {
        store_set("topic", &serde_json::Value::String(id.to_string()));
    }
}

// ---------- shell ----------
pub fn render_shell() {
    if let Some(b) = by_id("brand") {
        set_html(&b, "<svg class=\"bolt\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M13.5 1.5 3.8 13.6h6.6L9 22.5l10.2-12.6h-6.8z\"/></svg><b>Mechatronics Study Hub</b><span>ITE Nitec · NP Diploma</span>");
    }
    let tab = cur_tab();
    if let Some(t) = by_id("tabs") {
        let h: String = TABS
            .iter()
            .map(|(k, lb)| format!("<button role=\"tab\" data-tab=\"{}\" aria-selected=\"{}\">{}</button>", k, *k == tab, lb))
            .collect();
        set_html(&t, &h);
    }
    if let Some(tb) = by_id("themeBtn") {
        set_text(&tb, "Theme");
        set_attr(&tb, "title", "Switch light/dark");
    }
    if let Some(sb) = by_id("searchBtn") {
        set_attr(&sb, "title", "Search (Ctrl+K or /)");
        set_attr(&sb, "aria-label", "Search");
        if let Some(sp) = qs(&sb, "span") {
            set_text(&sp, "Search");
        }
    }
    let lg = lang();
    for b in dqsa("#langSeg button") {
        set_attr(&b, "aria-pressed", if attr(&b, "data-lang") == lg.code() { "true" } else { "false" });
    }
    if let Some(body) = doc().body() {
        toggle_class(&body, "show-zh", lg != Lang::En);
    }
    if let Some(de) = doc().document_element() {
        set_attr(&de, "lang", if lg == Lang::Zh { "zh-Hans" } else { "en" });
    }
    if let Some(h) = qs(&doc().document_element().unwrap(), "header.top") {
        i18n::translate(&h);
    }
}

pub fn set_language(code: &str) {
    let l = Lang::parse(code);
    if l == lang() {
        return;
    }
    set_lang(l);
    store_set("lang", &serde_json::Value::String(l.code().into()));
    render_shell();
    let t = cur_tab();
    go(&t, GoOpts { keep_scroll: true });
}

pub fn toggle_theme() {
    let r = doc().document_element().unwrap();
    let cur = r.get_attribute("data-theme").filter(|s| !s.is_empty()).unwrap_or_else(|| {
        let light = win().match_media("(prefers-color-scheme: light)").ok().flatten().map(|m| m.matches()).unwrap_or(false);
        if light { "light".into() } else { "dark".into() }
    });
    let n = if cur == "dark" { "light" } else { "dark" };
    set_attr(&r, "data-theme", n);
    store_set("theme", &serde_json::Value::String(n.into()));
}

// ---------- boot ----------
#[wasm_bindgen]
pub async fn boot(content_url: String) -> Result<(), JsValue> {
    let text = fetch_text(&content_url).await?;
    boot_with(&text)
}

/// Start from content text that the host already has (the desktop app passes its cached copy).
#[wasm_bindgen]
pub fn boot_with(text: &str) -> Result<(), JsValue> {
    let mut content: Content = serde_json::from_str(text).map_err(|e| JsValue::from_str(&format!("content.json: {}", e)))?;
    content.finish();
    let leaked: &'static Content = Box::leak(Box::new(content));
    CONTENT.with(|c| {
        let _ = c.set(leaked);
    });
    let lg = store_get("lang").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "both".into());
    set_lang(Lang::parse(&lg));
    let set_of = |k: &str| -> IndexSet<String> {
        store_get(k)
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect()
    };
    let best = store_get("best").and_then(|v| Some((v.get("s")?.as_f64()?, v.get("n")?.as_f64()?)));
    let st = St {
        tab: "home".into(),
        topic: store_get("topic").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "d1".into()),
        done: set_of("done"),
        known: set_of("known"),
        best,
        src_only: store_get("srconly").and_then(|v| v.as_bool()).unwrap_or(false),
        card: Default::default(),
        quiz: Default::default(),
        fx: Default::default(),
        xs: Default::default(),
        lab: lab::LabSt::load(),
        nb: notebook::NbSt::load(),
        nav: nav::NavSt::load(),
        sx: Default::default(),
        calc_seg: Default::default(),
    };
    STATE.with(|s| *s.borrow_mut() = Some(st));

    // theme
    let t = store_get("theme").and_then(|v| v.as_str().map(String::from));
    set_attr(&doc().document_element().unwrap(), "data-theme", t.as_deref().unwrap_or("dark"));

    i18n::install_observer();
    events::install();
    notebook::install();
    nav::install();
    search::install();
    render_shell();
    nav::header_metrics();
    notebook::init();
    let h = win().location().hash().unwrap_or_default();
    let h = h.trim_start_matches('#');
    let tab = if TABS.iter().any(|(k, _)| *k == h) { h.to_string() } else { "home".to_string() };
    go(&tab, GoOpts::default());
    Ok(())
}

async fn fetch_text(url: &str) -> Result<String, JsValue> {
    let resp = wasm_bindgen_futures::JsFuture::from(win().fetch_with_str(url)).await?;
    let resp: web_sys::Response = resp.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!("{} → HTTP {}", url, resp.status())));
    }
    let t = wasm_bindgen_futures::JsFuture::from(resp.text()?).await?;
    Ok(t.as_string().unwrap_or_default())
}

#[wasm_bindgen]
pub fn content_version() -> String {
    c().version.clone().unwrap_or_default()
}
