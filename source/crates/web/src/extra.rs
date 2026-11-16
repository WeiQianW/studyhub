//! Extra tab: library catalogue and the book / subject study guides.
use crate::util::*;
use crate::*;
use web_sys::Element;

#[derive(Default)]
pub struct XSt {
    pub cat: String,
    pub q: String,
}

fn ui(k: &str) -> String {
    c().ui.get(k).cloned().unwrap_or_default()
}

pub fn render() {
    let c = c();
    let tot: f64 = c.libcat.iter().map(|r| r.3).sum();
    let mut cnt: std::collections::HashMap<&str, f64> = Default::default();
    for r in &c.libcat {
        *cnt.entry(r.0.as_str()).or_insert(0.0) += r.3;
    }
    let gcard = |t: &data::Topic| -> String {
        format!(
            "<button class=\"card xguide\" data-topic=\"{}\"><span class=\"eyebrow\">{}</span><b>{}</b><span class=\"muted\">{}{}</span></button>",
            t.id,
            wk(t),
            tt(t),
            l(&format!("{} sections", t.sections.len()), &format!("{} 节", t.sections.len())),
            t.worked
                .as_ref()
                .map(|w| l(&format!(" · {} worked questions", w.len()), &format!(" · {} 道例题", w.len())))
                .unwrap_or_default()
        )
    };
    let xt: Vec<&data::Topic> = c.topics.iter().filter(|t| t.module == "xl").collect();
    let guides: String = xt.iter().filter(|t| t.wk.starts_with("Lib")).map(|t| gcard(t)).collect();
    let books: String = xt.iter().filter(|t| !t.wk.starts_with("Lib")).map(|t| gcard(t)).collect();
    let (cat, q) = with(|s| (s.xs.cat.clone(), s.xs.q.clone()));
    let mut chips = format!("<button class=\"xchip\" data-cat=\"\" aria-pressed=\"{}\">{} · {}</button>", cat.is_empty(), lp("All", "全部"), num_str(tot));
    for (k, g) in &c.libg {
        chips.push_str(&format!(
            "<button class=\"xchip\" data-cat=\"{}\" aria-pressed=\"{}\">{} · {}</button>",
            k,
            cat == *k,
            lp(g[0].as_str().unwrap_or(""), g[1].as_str().unwrap_or("")),
            num_str(*cnt.get(k.as_str()).unwrap_or(&0.0))
        ));
    }
    let tots = num_str(tot);
    let h = format!(
        "<div class=\"xlib\">\n   <section class=\"card\"><div class=\"eyebrow\">{eb}</div><h1 style=\"margin:8px 0 10px\">{h1}</h1>\n   <p class=\"muted\">{intro}</p>\n   <p class=\"note\" data-tr=\"1\">{note}</p></section>\n   <h2 class=\"xh\">{bh}</h2><p class=\"muted\" style=\"margin:-8px 0 0\">{bp}</p><div class=\"xguides\">{books}</div><h2 class=\"xh\">{sh}</h2><div class=\"xguides\">{guides}</div>\n   <h2 class=\"xh\">{ch}</h2>\n   <div class=\"card xcat\"><div class=\"xbar\"><input id=\"xq\" type=\"search\" value=\"{qv}\" placeholder=\"{ph}\" aria-label=\"{al}\"><span id=\"xn\" class=\"muted\"></span></div>\n   <div class=\"xchips\" data-tr=\"1\">{chips}</div><div id=\"xlist\"></div></div></div>",
        eb = l("Extra · beyond the course", "拓展 · 课程之外"),
        h1 = l("Electrical engineering library", "电气工程资料库"),
        intro = l(&ui("extra_intro_en").replace("{tot}", &tots), &ui("extra_intro_zh").replace("{tot}", &tots)),
        note = l(
            "Chinese practice follows GB standards, which can differ from Singapore (SS 638, IEC). For anything you do in the workshop, follow your lecturers and the Singapore rules.",
            "中国的做法依据 GB 标准，可能与新加坡（SS 638、IEC）不同。凡是在车间里实际操作的事情，请以讲师和新加坡的规定为准。"
        ),
        bh = l("Book notes — chapter by chapter", "读书笔记——逐章"),
        bp = l(&ui("extra_books_en"), &ui("extra_books_zh")),
        books = books,
        sh = l("Subject overviews", "主题概览"),
        guides = guides,
        ch = l("Book catalogue", "书目"),
        qv = q.replace('"', "&quot;"),
        ph = lp("Search title in English or Chinese", "用中文或英文搜索书名"),
        al = lp("Search the catalogue", "搜索书目"),
        chips = chips
    );
    set_html(&app(), &h);
    draw();
}

fn draw() {
    let c = c();
    let (cat, q) = with(|s| (s.xs.cat.clone(), js_trim(&s.xs.q).to_lowercase()));
    let rows: Vec<&(String, String, String, f64)> = c
        .libcat
        .iter()
        .filter(|r| (cat.is_empty() || r.0 == cat) && (q.is_empty() || r.1.to_lowercase().contains(&q) || r.2.to_lowercase().contains(&q)))
        .collect();
    let books: f64 = rows.iter().map(|r| r.3).sum();
    if let Some(n) = by_id("xn") {
        set_text(
            &n,
            &if lang() == Lang::Zh { format!("{} 个书名 · {} 本", rows.len(), num_str(books)) } else { format!("{} titles · {} books", rows.len(), num_str(books)) },
        );
    }
    let mut h = String::new();
    for (k, g) in &c.libg {
        let rs: Vec<_> = rows.iter().filter(|r| r.0 == *k).collect();
        if rs.is_empty() {
            continue;
        }
        let guide = g.get(2).and_then(|x| x.as_str());
        h.push_str(&format!(
            "<div class=\"xgrp\"><h3>{}{}</h3><div class=\"tbl\"><table><thead><tr><th>{}</th><th>{}</th><th class=\"xnum\">{}</th></tr></thead><tbody>{}</tbody></table></div></div>",
            l(g[0].as_str().unwrap_or(""), g[1].as_str().unwrap_or("")),
            guide.map(|gd| format!(" <button class=\"btn ghost sm\" data-topic=\"{}\">{}</button>", gd, lp("Open study guide", "打开学习指南"))).unwrap_or_default(),
            lp("Chinese title", "中文书名"),
            lp("English title", "英文书名"),
            lp("Books", "本数"),
            rs.iter().map(|r| format!("<tr><td lang=\"zh-Hans\">{}</td><td>{}</td><td class=\"xnum\">{}</td></tr>", esc(&r.1), esc(&r.2), num_str(r.3))).collect::<String>()
        ));
    }
    if h.is_empty() {
        h = format!("<p class=\"muted\">{}</p>", lp("No titles match.", "没有匹配的书名。"));
    }
    if let Some(e) = by_id("xlist") {
        set_html(&e, &h);
    }
}

pub fn on_click(t: &Element) -> bool {
    if let Some(b) = closest(t, "#app .xchip") {
        let cat = attr(&b, "data-cat");
        with(|s| s.xs.cat = cat.clone());
        for x in dqsa("#app .xchip") {
            set_attr(&x, "aria-pressed", if attr(&x, "data-cat") == cat { "true" } else { "false" });
        }
        draw();
        return true;
    }
    if let Some(b) = closest(t, "#app [data-topic]") {
        if closest(&b, "#xlist").is_some() || closest(&b, ".xguides").is_some() {
            set_topic(&attr(&b, "data-topic"), true);
            go("learn", GoOpts::default());
            return true;
        }
    }
    false
}
pub fn on_input(t: &Element) {
    if t.id() == "xq" {
        let v = input_value(t);
        with(|s| s.xs.q = v);
        draw();
    }
}
