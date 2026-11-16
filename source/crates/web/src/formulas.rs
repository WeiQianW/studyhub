//! Formula Sheet tab.
use crate::util::*;
use crate::*;
use serde_json::Value;
use std::collections::BTreeSet;
use web_sys::Element;

pub struct FxSt {
    pub g: String,
    pub q: String,
    pub m: String,
    pub open: BTreeSet<usize>,
}
impl Default for FxSt {
    fn default() -> Self {
        FxSt { g: "All".into(), q: String::new(), m: "all".into(), open: BTreeSet::new() }
    }
}

fn fs(f: &[Value], i: usize) -> &str {
    f.get(i).and_then(|x| x.as_str()).unwrap_or("")
}
fn mod_of(f: &[Value]) -> String {
    c().topic(fs(f, 7)).map(|t| t.module.clone()).unwrap_or_else(|| "other".into())
}
fn zf(i: usize) -> Option<&'static Value> {
    c().zf.get(i).filter(|v| v.is_array())
}
fn zfs(i: usize, j: usize) -> Option<&'static str> {
    zf(i).and_then(|z| z.get(j)).and_then(|x| x.as_str())
}
fn zfa(i: usize, j: usize, k: usize) -> Option<&'static str> {
    zf(i).and_then(|z| z.get(j)).and_then(|x| x.get(k)).and_then(|x| x.as_str())
}
fn zg(g: &str) -> Option<&'static str> {
    c().zg.get(g).map(|s| s.as_str())
}

fn mods() -> Vec<String> {
    c().mod_order.iter().filter(|m| c().fx.iter().any(|f| mod_of(f) == **m)).cloned().collect()
}

pub fn render() {
    let c = c();
    let ms = mods();
    with(|s| {
        if s.fx.m != "all" && !ms.contains(&s.fx.m) {
            s.fx.m = "all".into();
        }
        if s.fx.m.is_empty() {
            s.fx.m = "all".into();
        }
    });
    let (m, q) = with(|s| (s.fx.m.clone(), s.fx.q.clone()));
    let cnt = |mm: &str| c.fx.iter().filter(|f| mm == "all" || mod_of(f) == mm).count();
    let zh = lang() == Lang::Zh;
    let mut segs = vec!["all".to_string()];
    segs.extend(ms.iter().cloned());
    let h = format!(
        "<div class=\"fxhead\">\n   <div><h2>{}</h2><p class=\"muted\">{}</p></div>\n   <div class=\"fxtools\"><input type=\"search\" id=\"fxq\" placeholder=\"{}\" value=\"{}\" aria-label=\"Search formulas\">\n   <button class=\"btn ghost sm\" id=\"fxall\"></button></div></div>\n  <div class=\"seg fxmods\" role=\"tablist\">{}</div>\n  <div class=\"chips fxchips\" id=\"fxchips\"></div>\n  <div id=\"fxlist\"></div>\n  <details class=\"card fxref\"><summary>{}</summary><div class=\"reader\" style=\"padding:0\">{}</div></details>",
        l("Formula Sheet", "公式表"),
        l(
            &format!("{} formulas. Tap a row to see what every symbol means, a worked example and common mistakes.", c.fx.len()),
            &format!("共 {} 条公式。点击任一行可查看每个符号的含义、例题和常见错误。", c.fx.len())
        ),
        if zh { "搜索公式，如 转差率、转矩" } else { "Search, e.g. slip, torque, gear" },
        esc(&q),
        segs.iter()
            .map(|x| format!(
                "<button data-m=\"{}\" aria-pressed=\"{}\">{}<span class=\"fxn\">{}</span></button>",
                x,
                m == *x,
                if x == "all" { lp("All", "全部") } else { c.mods[x].code.clone() },
                cnt(x)
            ))
            .collect::<String>(),
        l("Quick reference tables (drives & PLC)", "速查表（驱动与 PLC）"),
        c.qref.get(lang().code()).cloned().unwrap_or_default()
    );
    set_html(&app(), &h);
    list();
}

fn row(f: &[Value], i: usize, open: bool) -> String {
    let title = fs(f, 1);
    let formula = fs(f, 2);
    let sym = f.get(3).and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let mean = fs(f, 4);
    let ex = fs(f, 5);
    let tips = f.get(6).and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let topic = fs(f, 7);
    let calc = f.get(8).and_then(|x| x.as_str()).filter(|s| !s.is_empty());
    let lg = lang();
    let fm = match zfs(i, 5).filter(|s| !s.is_empty()) {
        Some(z5) if lg != Lang::En => {
            if lg == Lang::Zh {
                z5.to_string()
            } else {
                format!("{}{}", formula, z5)
            }
        }
        _ => formula.to_string(),
    };
    let t = c().topic(topic);
    format!(
        "<details class=\"fxrow\" id=\"fx-{i}\" data-i=\"{i}\" data-hk=\"fx\" data-sec=\"{i}\" {open}><summary><span class=\"fxt\">{title}</span><span class=\"fxf\">{fm}</span></summary>\n  <div class=\"fxbody\">\n   <div class=\"tbl\"><table class=\"sym\"><tr><th>{s1}</th><th>{s2}</th><th>{s3}</th></tr>{syms}</table></div>\n   <div class=\"fxcols\"><div><h4>{h1}</h4><p>{mean}</p><h4>{h2}</h4><ul>{tips}</ul></div>\n   <div><h4>{h3}</h4><pre class=\"fxex\">{ex}</pre></div></div>\n   <div class=\"fxbtns\">{cb}<button class=\"chipbtn\" data-t=\"{topic}\">{wk} · {tt}</button></div>\n  </div></details>",
        i = i,
        open = if open { "open" } else { "" },
        title = tl(title, zfs(i, 0)),
        fm = fm,
        s1 = l("Symbol", "符号"),
        s2 = l("Meaning", "含义"),
        s3 = l("Unit", "单位"),
        syms = sym
            .iter()
            .enumerate()
            .map(|(j, s)| format!(
                "<tr><td class=\"mono\">{}</td><td>{}</td><td class=\"mono\">{}</td></tr>",
                js_elem(&s[0]),
                tl(&js_elem(&s[1]), zfa(i, 1, j)),
                js_elem(&s[2])
            ))
            .collect::<String>(),
        h1 = l("What it means", "含义"),
        mean = tl(mean, zfs(i, 2)),
        h2 = l("Watch out", "注意"),
        tips = tips.iter().enumerate().map(|(j, t)| format!("<li>{}</li>", tl(&js_elem(t), zfa(i, 4, j)))).collect::<String>(),
        h3 = l("Worked example", "例题"),
        ex = tlb(ex, zfs(i, 3)),
        cb = calc.map(|cc| format!("<button class=\"btn ghost sm\" data-calc=\"{}\">{}</button>", cc, lp("Open in calculator", "在计算器中打开"))).unwrap_or_default(),
        topic = topic,
        wk = t.map(wk).unwrap_or_default(),
        tt = t.map(ttp).unwrap_or_default()
    )
}

/// The original's search haystack: `f.slice(0,7).join(' ') + f[3].flat().join(' ') + ' ' + ZF[i].flat().join(' ') + ' ' + ZG[f[0]]`
fn haystack(f: &[Value], i: usize) -> String {
    let a = js_join(&f[..7.min(f.len())], " ");
    let flat3: Vec<Value> = f.get(3).and_then(|x| x.as_array()).map(|a| a.iter().flat_map(|x| x.as_array().cloned().unwrap_or_else(|| vec![x.clone()])).collect()).unwrap_or_default();
    let zflat: Vec<Value> = zf(i)
        .and_then(|z| z.as_array())
        .map(|a| a.iter().flat_map(|x| x.as_array().cloned().unwrap_or_else(|| vec![x.clone()])).collect())
        .unwrap_or_default();
    let s = format!("{}{} {} {}", a, js_join(&flat3, " "), js_join(&zflat, " "), zg(fs(f, 0)).unwrap_or(""));
    strip_tags(&s.to_lowercase(), "")
}

fn list() {
    let c = c();
    let ms = mods();
    let (m, q, open) = with(|s| (s.fx.m.clone(), js_trim(&s.fx.q).to_lowercase(), s.fx.open.clone()));
    let in_mod: Vec<(usize, &Vec<Value>)> = c.fx.iter().enumerate().filter(|(_, f)| m == "all" || mod_of(f) == m).collect();
    let mut groups: Vec<String> = vec![];
    for (_, f) in &in_mod {
        let g = fs(f, 0).to_string();
        if !groups.contains(&g) {
            groups.push(g);
        }
    }
    with(|s| {
        if s.fx.g != "All" && !groups.contains(&s.fx.g) {
            s.fx.g = "All".into();
        }
    });
    let g = with(|s| s.fx.g.clone());
    let chips = if m != "all" && groups.len() > 1 {
        let mut all = vec!["All".to_string()];
        all.extend(groups.iter().cloned());
        all.iter()
            .map(|x| {
                let lab = if x == "All" {
                    lp("All topics", "全部主题")
                } else if zg(x) == Some(x.as_str()) {
                    x.clone()
                } else {
                    tlp(x, zg(x))
                };
                format!("<button class=\"chipbtn\" data-g=\"{}\" aria-pressed=\"{}\">{}</button>", x, g == *x, lab)
            })
            .collect::<String>()
    } else {
        String::new()
    };
    if let Some(e) = by_id("fxchips") {
        set_html(&e, &chips);
    }
    let items: Vec<(usize, &Vec<Value>)> = in_mod.into_iter().filter(|(i, f)| (g == "All" || fs(f, 0) == g) && (q.is_empty() || haystack(f, *i).contains(&q))).collect();
    let show: Vec<String> = (if m == "all" { ms.clone() } else { vec![m.clone()] }).into_iter().filter(|mm| items.iter().any(|(_, f)| mod_of(f) == *mm)).collect();
    let html = if !items.is_empty() {
        show.iter()
            .map(|mm| {
                let its: Vec<&(usize, &Vec<Value>)> = items.iter().filter(|(_, f)| mod_of(f) == *mm).collect();
                let mut gs: Vec<String> = vec![];
                for (_, f) in &its {
                    let g = fs(f, 0).to_string();
                    if !gs.contains(&g) {
                        gs.push(g);
                    }
                }
                let term = c.term_of(mm);
                format!(
                    "<section class=\"fxmod\"><header class=\"fxmodh\"><span class=\"scode\">{}</span><h3>{}</h3><span class=\"muted fxsem\">{}</span></header>\n   {}</section>",
                    c.mods[mm].code,
                    mn(mm),
                    lp(&term.name, &term.zh),
                    gs.iter()
                        .map(|g| format!(
                            "<div class=\"fxgrp\"><div class=\"fxgh\">{}</div><div class=\"fxrows\">{}</div></div>",
                            if zg(g) == Some(g.as_str()) { g.clone() } else { tl(g, zg(g)) },
                            its.iter().filter(|(_, f)| fs(f, 0) == g).map(|(i, f)| row(f, *i, open.contains(i))).collect::<String>()
                        ))
                        .collect::<String>()
                )
            })
            .collect::<String>()
    } else {
        format!("<p class=\"muted\">{}</p>", l("No formulas match that search.", "没有符合的公式。"))
    };
    if let Some(e) = by_id("fxlist") {
        set_html(&e, &html);
    }
    if !q.is_empty() && items.len() <= 3 {
        for d in dqsa("#fxlist details.fxrow") {
            set_details_open(&d, true);
        }
    }
    sync_all();
    notebook::apply_marks();
    notebook::render();
}

fn sync_all() {
    let ds = dqsa("#fxlist details.fxrow");
    let all_open = !ds.is_empty() && ds.iter().all(details_open);
    if let Some(b) = by_id("fxall") {
        let zh = lang() == Lang::Zh;
        set_text(&b, if all_open { if zh { "全部收起" } else { "Collapse all" } } else if zh { "全部展开" } else { "Expand all" });
        set_attr(&b, "data-mode", if all_open { "close" } else { "open" });
    }
}

pub fn on_click(t: &Element) -> bool {
    if let Some(b) = closest(t, "#fxchips [data-g]") {
        let g = attr(&b, "data-g");
        with(|s| s.fx.g = g);
        list();
        return true;
    }
    if let Some(b) = closest(t, "#fxlist [data-t]") {
        set_topic(&attr(&b, "data-t"), true);
        go("learn", GoOpts::default());
        return true;
    }
    if let Some(b) = closest(t, "#fxlist [data-calc]") {
        let id = attr(&b, "data-calc");
        go("calc", GoOpts::default());
        timeout(60, move || {
            if let Some(el) = by_id(&id) {
                scroll_center(&el);
                add_class(&el, "flashhl");
                timeout(1600, move || rm_class(&el, "flashhl"));
            }
        });
        return true;
    }
    if let Some(b) = closest(t, "#fxall") {
        let open = attr(&b, "data-mode") == "open";
        for d in dqsa("#fxlist details.fxrow") {
            set_details_open(&d, open);
            let i: usize = attr(&d, "data-i").parse().unwrap_or(0);
            with(|s| {
                if open {
                    s.fx.open.insert(i);
                } else {
                    s.fx.open.remove(&i);
                }
            });
        }
        sync_all();
        return true;
    }
    if let Some(b) = closest(t, "#app .fxmods [data-m]") {
        let m = attr(&b, "data-m");
        with(|s| {
            s.fx.m = m;
            s.fx.g = "All".into();
        });
        for x in dqsa("#app .fxmods [data-m]") {
            set_attr(&x, "aria-pressed", if x.is_same_node(Some(&b)) { "true" } else { "false" });
        }
        list();
        return true;
    }
    false
}
pub fn scroll_center(el: &Element) {
    let o = web_sys::ScrollIntoViewOptions::new();
    o.set_block(web_sys::ScrollLogicalPosition::Center);
    el.scroll_into_view_with_scroll_into_view_options(&o);
}
pub fn on_input(t: &Element) {
    if t.id() == "fxq" {
        let v = input_value(t);
        with(|s| s.fx.q = v);
        list();
    }
}
pub fn on_toggle(t: &Element) {
    if t.matches("#fxlist details.fxrow").unwrap_or(false) {
        let i: usize = attr(t, "data-i").parse().unwrap_or(0);
        let o = details_open(t);
        with(|s| {
            if o {
                s.fx.open.insert(i);
            } else {
                s.fx.open.remove(&i);
            }
        });
        sync_all();
    }
}
