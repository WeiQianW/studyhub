//! Search everything (Ctrl/⌘+K or /): topics, sections, terms, formulas, calculators.
use crate::util::*;
use crate::*;
use wasm_bindgen::JsCast;
use web_sys::Element;

#[derive(Clone)]
enum Act {
    Topic(String, Option<i64>),
    Formula(usize),
    Calc(String),
}
#[derive(Clone)]
struct Item {
    kind: &'static str,
    rank: u32,
    en: String,
    zh: String,
    ctx: String,
    act: Act,
    body: String,
    lc: String,
    blc: String,
}

#[derive(Default)]
pub struct SxSt {
    idx: Option<Vec<Item>>,
    sel: usize,
    res: Vec<usize>,
}

fn strip(h: &str) -> String {
    let s = strip_blocks(h, "<svg", "</svg>", " ");
    let s = strip_tags(&s, " ");
    let s = s.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&").replace("&nbsp;", " ");
    let mut out = String::with_capacity(s.len());
    let mut sp = false;
    for ch in s.chars() {
        if is_js_space(ch) {
            if !sp {
                out.push(' ');
            }
            sp = true;
        } else {
            out.push(ch);
            sp = false;
        }
    }
    js_trim(&out).to_string()
}

fn build() -> Vec<Item> {
    let c = c();
    let mut x = vec![];
    let mut add = |kind: &'static str, rank: u32, en: String, zh: String, ctx: String, act: Act, body: String| {
        let lc = format!("{} \u{1} {}", en, zh).to_lowercase();
        let blc = body.to_lowercase();
        x.push(Item { kind, rank, en, zh, ctx, act, body, lc, blc });
    };
    for t in &c.topics {
        let empty = Default::default();
        let z = c.zt(&t.id).unwrap_or(&empty);
        let ctx = format!("{} · {}", c.mods[&t.module].code, t.wk);
        add("topic", 0, strip(&t.title), t.zh.clone(), ctx.clone(), Act::Topic(t.id.clone(), None), String::new());
        for (i, s) in t.sections.iter().enumerate() {
            let zh0 = data::zget(&z.s, i, 0).unwrap_or("");
            let zh1 = data::zget(&z.s, i, 1).unwrap_or("");
            add(
                "section",
                1,
                strip(&s.h),
                strip(zh0),
                format!("{} · {}", ctx, strip(&t.title)),
                Act::Topic(t.id.clone(), Some(i as i64)),
                format!("{} \u{1} {}", strip(&s.b), strip(zh1)),
            );
        }
        for term in t.terms.iter().flatten() {
            add(
                "term",
                2,
                strip(term.first().map(|s| s.as_str()).unwrap_or("")),
                strip(term.get(1).map(|s| s.as_str()).unwrap_or("")),
                format!("{} · {}", ctx, strip(&t.title)),
                Act::Topic(t.id.clone(), Some(-1)),
                String::new(),
            );
        }
    }
    for (i, f) in c.fx.iter().enumerate() {
        let zf0 = c.zf.get(i).and_then(|z| z.get(0)).and_then(|v| v.as_str()).unwrap_or("").to_string();
        add(
            "formula",
            1,
            strip(f.get(1).and_then(|v| v.as_str()).unwrap_or("")),
            zf0,
            strip(f.first().and_then(|v| v.as_str()).unwrap_or("")),
            Act::Formula(i),
            strip(f.get(4).and_then(|v| v.as_str()).unwrap_or("")),
        );
    }
    for cm in &c.calcs {
        add(
            "calc",
            1,
            cm.title.clone(),
            zd(&cm.title).unwrap_or("").to_string(),
            c.topic(&cm.topic).map(|t| c.mods[&t.module].code.clone()).unwrap_or_default(),
            Act::Calc(cm.id.clone()),
            String::new(),
        );
    }
    x
}

fn find(idx: &[Item], q: &str) -> Vec<usize> {
    let q = js_trim(q).to_lowercase();
    if q.is_empty() {
        return vec![];
    }
    let words: Vec<&str> = q.split(|c: char| is_js_space(c)).filter(|w| !w.is_empty()).collect();
    let mut out: Vec<(u32, usize)> = vec![];
    for (i, x) in idx.iter().enumerate() {
        let mut sc: i64 = -1;
        if words.iter().all(|w| x.lc.contains(w)) {
            sc = (x.rank * 10 + if x.lc.starts_with(&q) { 0 } else if x.lc.contains(&q) { 2 } else { 4 }) as i64;
        } else if !x.blc.is_empty() && words.iter().all(|w| x.blc.contains(w) || x.lc.contains(w)) {
            sc = 60 + if x.blc.contains(&q) { 0 } else { 5 };
        }
        if sc >= 0 {
            out.push((sc as u32, i));
        }
    }
    out.sort_by_key(|a| a.0); // stable, like Array.prototype.sort
    out.into_iter().take(40).map(|x| x.1).collect()
}

/// Character-based slice like String.prototype.slice on UTF-16 indices.
fn u16slice(s: &str, a: usize, b: usize) -> String {
    let v: Vec<u16> = s.encode_utf16().collect();
    let b = b.min(v.len());
    if a >= b {
        return String::new();
    }
    String::from_utf16_lossy(&v[a..b])
}
fn u16find(hay: &str, needle: &str) -> Option<usize> {
    let i = hay.find(needle)?;
    Some(hay[..i].encode_utf16().count())
}
fn snip(x: &Item, q: &str) -> String {
    let first = q.split(|c: char| is_js_space(c)).next().unwrap_or("");
    let i = match u16find(&x.blc, first) {
        Some(i) => i,
        None => return String::new(),
    };
    let a = i.saturating_sub(40);
    format!("{}{}…", if a > 0 { "…" } else { "" }, esc(&u16slice(&x.body, a, i + 90)).replace('\u{1}', "·"))
}

fn dlg() -> Element {
    by_id("sx").unwrap()
}
fn is_open() -> bool {
    dlg().has_attribute("open")
}

pub fn install() {
    let d = doc().create_element("dialog").unwrap();
    d.set_id("sx");
    set_attr(&d, "data-tr", "1");
    let _ = doc().body().unwrap().append_child(&d);
}

pub fn open() {
    if is_open() {
        return;
    }
    if with(|s| s.sx.idx.is_none()) {
        let b = build();
        with(|s| s.sx.idx = Some(b));
    }
    let zh = lang() == Lang::Zh;
    let d = dlg();
    set_html(
        &d,
        &format!(
            "<div class=\"sxbox\"><input id=\"sxq\" type=\"search\" autocomplete=\"off\" spellcheck=\"false\" placeholder=\"{}\" aria-label=\"{}\"><div id=\"sxl\" role=\"listbox\"></div><div class=\"sxfoot\">{}</div></div>",
            if zh { "搜索主题、小节、术语、公式、计算器…" } else { "Search topics, sections, terms, formulas, calculators…" },
            if zh { "搜索" } else { "Search" },
            if zh { "↑↓ 选择 · Enter 打开 · Esc 关闭" } else { "↑↓ to move · Enter to open · Esc to close" }
        ),
    );
    let dd: &web_sys::HtmlDialogElement = d.unchecked_ref();
    if dd.show_modal().is_err() {
        set_attr(&d, "open", "");
    }
    with(|s| s.sx.sel = 0);
    draw();
    if let Some(i) = by_id("sxq") {
        let _ = hel(&i).focus();
    }
}

fn draw() {
    let inp = match by_id("sxq") {
        Some(i) => i,
        None => return,
    };
    let q = js_trim(&input_value(&inp)).to_lowercase();
    let zh = lang() == Lang::Zh;
    let lg = lang();
    let h = with(|s| {
        let idx = s.sx.idx.as_ref().unwrap();
        s.sx.res = find(idx, &q);
        if s.sx.sel >= s.sx.res.len() {
            s.sx.sel = 0;
        }
        if q.is_empty() {
            return format!("<p class=\"sxhint\">{}</p>", if zh { "输入中文或英文关键词，例如：级联、PWM、转差率、MoveL" } else { "Type a keyword in English or Chinese, e.g. cascade, PWM, slip, MoveL" });
        }
        if s.sx.res.is_empty() {
            return format!("<p class=\"sxhint\">{}</p>", if zh { "没有找到结果" } else { "No results" });
        }
        let first = q.split(|c: char| is_js_space(c)).next().unwrap_or("");
        s.sx
            .res
            .iter()
            .enumerate()
            .map(|(i, &xi)| {
                let x = &idx[xi];
                let main = if zh && !x.zh.is_empty() { &x.zh } else { &x.en };
                let alt = if zh { x.en.as_str() } else if lg == Lang::Both { x.zh.as_str() } else { "" };
                let sn = if !x.body.is_empty() && !x.lc.contains(first) { snip(x, &q) } else { String::new() };
                let kind = match x.kind {
                    "topic" => ["Topic", "主题"],
                    "section" => ["Section", "小节"],
                    "term" => ["Term", "术语"],
                    "formula" => ["Formula", "公式"],
                    _ => ["Calculator", "计算器"],
                };
                format!(
                    "<button role=\"option\" class=\"sxi\" data-i=\"{}\" aria-selected=\"{}\"><span class=\"sxk\">{}</span><span class=\"sxm\"><b>{}</b>{}{}</span><span class=\"sxc\">{}</span></button>",
                    i,
                    i == s.sx.sel,
                    kind[if zh { 1 } else { 0 }],
                    esc(main),
                    if !alt.is_empty() && alt != main { format!("<i>{}</i>", esc(alt)) } else { String::new() },
                    if !sn.is_empty() { format!("<small>{}</small>", sn) } else { String::new() },
                    esc(&x.ctx)
                )
            })
            .collect()
    });
    if let Some(l) = by_id("sxl") {
        set_html(&l, &h);
    }
}

pub fn close() {
    if is_open() {
        let d = dlg();
        let dd: &web_sys::HtmlDialogElement = d.unchecked_ref();
        dd.close();
        d.remove_attribute("open").ok();
    }
}

fn pick(i: usize) {
    let it = with(|s| s.sx.res.get(i).and_then(|&xi| s.sx.idx.as_ref().map(|v| v[xi].clone())));
    let x = match it {
        Some(x) => x,
        None => return,
    };
    close();
    match x.act {
        Act::Topic(t, s) => {
            set_topic(&t, true);
            go("learn", GoOpts::default());
            match s {
                Some(s) => {
                    let n = if s < 0 { qsa(&app(), ".reader>section").len() as i64 - 2 } else { s };
                    raf(move || {
                        if n >= 0 {
                            nav::sec_go(n as usize)
                        }
                    });
                }
                None => {
                    with(|st| {
                        st.nav.run += 1;
                        st.nav.lock = false;
                    });
                    scroll_to(0.0);
                }
            }
        }
        Act::Formula(f) => {
            with(|s| {
                s.fx.m = "all".into();
                s.fx.g = "All".into();
                s.fx.q.clear();
                s.fx.open.insert(f);
            });
            go("formulas", GoOpts::default());
            timeout(80, move || {
                if let Some(el) = by_id(&format!("fx-{}", f)) {
                    with(|s| {
                        s.nav.run += 1;
                        s.nav.lock = false;
                    });
                    set_details_open(&el, true);
                    formulas::scroll_center(&el);
                    add_class(&el, "flashhl");
                    timeout(1600, move || rm_class(&el, "flashhl"));
                }
            });
        }
        Act::Calc(id) => {
            go("calc", GoOpts::default());
            timeout(80, move || {
                if let Some(el) = by_id(&id) {
                    with(|s| {
                        s.nav.run += 1;
                        s.nav.lock = false;
                    });
                    formulas::scroll_center(&el);
                    add_class(&el, "flashhl");
                    timeout(1600, move || rm_class(&el, "flashhl"));
                }
            });
        }
    }
}

/// Clicks inside the dialog. Returns true when handled.
pub fn on_click(t: &Element, e: &web_sys::MouseEvent) -> bool {
    if closest(t, "#searchBtn").is_some() {
        open();
        return true;
    }
    let d = dlg();
    if !d.contains(Some(t)) {
        return false;
    }
    if let Some(b) = closest(t, ".sxi") {
        let i: usize = attr(&b, "data-i").parse().unwrap_or(0);
        pick(i);
        return true;
    }
    if t.is_same_node(Some(&d)) {
        let _ = e;
        close();
    }
    true
}
pub fn on_input(t: &Element) -> bool {
    if t.id() == "sxq" {
        with(|s| s.sx.sel = 0);
        draw();
        return true;
    }
    false
}
/// keydown anywhere: shortcuts, plus arrow/enter inside the box.
pub fn on_keydown(e: &web_sys::KeyboardEvent, t: &Element) -> bool {
    let k = e.key();
    if t.id() == "sxq" {
        if k == "ArrowDown" || k == "ArrowUp" {
            e.prevent_default();
            let n = with(|s| s.sx.res.len());
            if n == 0 {
                return true;
            }
            let sel = with(|s| {
                s.sx.sel = (s.sx.sel + if k == "ArrowDown" { 1 } else { n - 1 }) % n;
                s.sx.sel
            });
            if let Some(l) = by_id("sxl") {
                for (i, b) in qsa(&l, ".sxi").iter().enumerate() {
                    set_attr(b, "aria-selected", if i == sel { "true" } else { "false" });
                }
                let ch = l.children();
                if let Some(c) = ch.item(sel as u32) {
                    let o = web_sys::ScrollIntoViewOptions::new();
                    o.set_block(web_sys::ScrollLogicalPosition::Nearest);
                    c.scroll_into_view_with_scroll_into_view_options(&o);
                }
            }
            return true;
        } else if k == "Enter" {
            e.prevent_default();
            let sel = with(|s| s.sx.sel);
            if with(|s| s.sx.res.len()) > sel {
                pick(sel);
            }
            return true;
        }
    }
    let tag = t.tag_name();
    let in_field = tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" || hel(t).is_content_editable();
    if (k == "k" || k == "K") && (e.ctrl_key() || e.meta_key()) {
        e.prevent_default();
        open();
        return true;
    } else if k == "/" && !in_field && !e.ctrl_key() && !e.meta_key() && !e.alt_key() {
        e.prevent_default();
        open();
        return true;
    }
    false
}
