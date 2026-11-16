//! Notes tab: sidebar, reader, recap blanks, figures, formula annotations, SCR explorer.
use crate::data::{zget, zrow};
use crate::util::*;
use crate::*;
use std::cell::Cell;
use web_sys::Element;

thread_local! {
    static SIDE_Y: Cell<f64> = Cell::new(ls_get("sh_side").map(|s| parse_float(&s)).filter(|v| v.is_finite()).unwrap_or(0.0));
}

fn side_open() -> Vec<String> {
    ls_get("sh_sideopen")
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

/// renderLearn with both wrappers of the original (scroll memory, then sidebar position).
pub fn render() {
    // sidebar wrapper (outer)
    if let Some(old) = qs(&app(), ".side") {
        SIDE_Y.with(|y| y.set(old.scroll_top() as f64));
    }
    // scroll-memory wrapper
    let (going, tab) = with(|s| (s.nav.going, s.tab.clone()));
    if going || tab != "learn" {
        render_core();
    } else if nav::key() == with(|s| s.nav.k.clone()).unwrap_or_default() {
        let an = if scroll_y() > 0.0 { nav::anchor() } else { None };
        render_core();
        if let Some(a) = an {
            with(|s| s.nav.lock = true);
            let run = nav::bump_run();
            nav::to_anchor(a, run, false);
        }
    } else {
        with(|s| s.nav.lock = true);
        render_core();
        nav::arrive(false);
    }
    // sidebar wrapper (after)
    if let Some(s) = qs(&app(), ".side") {
        let y = SIDE_Y.with(|y| y.get());
        s.set_scroll_top(y as i32);
        if let Some(cur) = qs(&s, "[aria-current=\"true\"]") {
            let sr = s.get_bounding_client_rect();
            let cr = cur.get_bounding_client_rect();
            if cr.top() < sr.top() || cr.bottom() > sr.bottom() {
                let v = s.scroll_top() as f64 + cr.top() - sr.top() - sr.height() / 3.0;
                s.set_scroll_top(v as i32);
            }
        }
    }
}

pub fn on_side_scroll(s: &Element) {
    let y = s.scroll_top() as f64;
    SIDE_Y.with(|v| v.set(y));
    ls_set("sh_side", &num_str(y.round()));
}

fn render_core() {
    let c = c();
    let topic = with(|s| s.topic.clone());
    let t = c.topic(&topic).unwrap_or(&c.topics[0]);
    let idx = c.by_id[&t.id];
    let prev = if idx > 0 { c.topics.get(idx - 1) } else { None };
    let next = c.topics.get(idx + 1);
    let so = side_open();
    let done = with(|s| s.done.clone());
    let lg = lang();

    let mut side = String::new();
    for p in c.programs.iter().filter(|p| !p.terms.is_empty()) {
        side.push_str(&format!("<div class=\"sprog\">{}</div>", l(&p.inst, &p.zinst)));
        for term in &p.terms {
            side.push_str(&format!("<div class=\"sterm\">{}</div>", l(&term.name, &term.zh)));
            for m in &term.mods {
                side.push_str(&format!(
                    "<details class=\"smod\" data-m=\"{}\" {}><summary><span class=\"scode\">{}</span> {}</summary>",
                    m,
                    if *m == t.module || so.contains(m) { "open" } else { "" },
                    c.mods[m].code,
                    mn(m)
                ));
                for x in c.topics.iter().filter(|x| x.module == *m) {
                    side.push_str(&format!(
                        "<button data-t=\"{}\" aria-current=\"{}\"><span class=\"done-dot {}\"></span><span class=\"wk\">{}</span><span>{}</span></button>",
                        x.id,
                        x.id == t.id,
                        if done.contains(&x.id) { "on" } else { "" },
                        wk(x),
                        tt(x)
                    ));
                }
                side.push_str("</details>");
            }
        }
    }
    for p in c.programs.iter().filter(|p| p.terms.is_empty()) {
        side.push_str(&format!(
            "<div class=\"sprog\">{}</div><p class=\"muted snote\">{}</p>",
            l(&p.inst, &p.zinst),
            tl(p.note.as_deref().unwrap_or(""), p.znote.as_deref())
        ));
    }

    let mut pick = String::from("<select class=\"mobile-pick\" id=\"pick\" aria-label=\"Choose topic\">");
    for m in &c.mod_order {
        let term = c.term_of(m);
        let md = &c.mods[m];
        pick.push_str(&format!("<optgroup label=\"{} · {} {}\">", lp(&term.name, &term.zh), md.code, lp(&md.name, &md.zh)));
        for x in c.topics.iter().filter(|x| x.module == *m) {
            pick.push_str(&format!(
                "<option value=\"{}\" {}>{} · {}{}</option>",
                x.id,
                if x.id == t.id { "selected" } else { "" },
                wk(x),
                ttp(x),
                if done.contains(&x.id) { " ✓" } else { "" }
            ));
        }
        pick.push_str("</optgroup>");
    }
    pick.push_str("</select>");

    let empty = Default::default();
    let z = c.zt(&t.id).unwrap_or(&empty);
    let worked = match &t.worked {
        Some(w) => format!(
            "<section><h3>Worked examples &amp; exercise answers</h3>{}</section>",
            w.iter()
                .enumerate()
                .map(|(i, w)| format!(
                    "<details class=\"ex\"><summary>{}</summary><div class=\"sol\">{}</div></details>",
                    tl(&w.q, zget(&z.w, i, 0)),
                    tlb(&w.s, zget(&z.w, i, 1))
                ))
                .collect::<String>()
        ),
        None => String::new(),
    };
    let rrow = |r0: &str, r1: &str, cls: &str| -> String {
        let parts: Vec<&str> = r0.split("___").collect();
        let ans: Vec<&str> = r1.split(" · ").collect();
        let mut h = format!("<div class=\"r {}\">", cls);
        for (i, p) in parts.iter().enumerate() {
            h.push_str(p);
            if i < parts.len() - 1 {
                let a = ans.get(i).filter(|s| !s.is_empty()).unwrap_or(&ans[0]);
                h.push_str(&format!("<button class=\"blank\" data-a=\"{}\">?</button>", a.replace('"', "&quot;")));
            }
        }
        h.push_str("</div>");
        h
    };
    let recap = match &t.recap {
        Some(rows) => {
            let mut h = String::new();
            for (i, r) in rows.iter().enumerate() {
                let zr = zrow(&z.r, i);
                match (lg, zr) {
                    (Lang::En, _) | (_, None) => h.push_str(&rrow(&r[0], &r[1], "")),
                    (Lang::Zh, Some(zr)) => h.push_str(&rrow(zr[0].as_deref().unwrap_or(""), zr[1].as_deref().unwrap_or(""), "")),
                    (Lang::Both, Some(zr)) => {
                        h.push_str(&rrow(&r[0], &r[1], ""));
                        h.push_str(&rrow(zr[0].as_deref().unwrap_or(""), zr[1].as_deref().unwrap_or(""), "zhr"));
                    }
                }
            }
            format!("<section><h3>Recap — tap a blank to reveal</h3><div class=\"recap\">{}</div></section>", h)
        }
        None => String::new(),
    };
    let terms = match &t.terms {
        Some(ts) => format!(
            "<section><h3>{}</h3><div class=\"terms\">{}</div>{}</section>",
            l("Key terms", "关键术语"),
            ts.iter().map(|x| format!("<span class=\"term\">{}<span class=\"zh\">{}</span></span>", x[0], x[1])).collect::<String>(),
            if lg == Lang::En {
                "<p class=\"muted\" style=\"font-size:13px;margin-top:8px\">Switch to 中文 or EN+中文 at the top to read everything in Chinese.</p>"
            } else {
                ""
            }
        ),
        None => String::new(),
    };
    let term = c.term_of(&t.module);
    let is_done = done.contains(&t.id);
    let toc = if t.sections.len() >= 4 {
        format!(
            "<nav class=\"toc\" data-tr=\"1\" aria-label=\"{}\"><span class=\"toch\">{}</span>{}</nav>",
            if lg == Lang::Zh { "本页内容" } else { "On this page" },
            if lg == Lang::Zh { "本页" } else { "On this page" },
            t.sections
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let h = if lg == Lang::Zh { zget(&z.s, i, 0).filter(|x| !x.is_empty()).unwrap_or(&s.h) } else { &s.h };
                    format!("<button type=\"button\" data-sec-go=\"{}\">{}. {}</button>", i, i + 1, h)
                })
                .collect::<String>()
        )
    } else {
        String::new()
    };
    let secs: String = t
        .sections
        .iter()
        .enumerate()
        .map(|(i, s)| format!("<section><h3>{}</h3>{}</section>", tl(&s.h, zget(&z.s, i, 0)), tlb(&s.b, zget(&z.s, i, 1))))
        .collect();
    let pager_prev = match prev {
        Some(p) => format!("<button class=\"btn ghost\" data-t=\"{}\">← {} {}</button>", p.id, wk(p), ttp(p)),
        None => "<span></span>".into(),
    };
    let pager_next = match next {
        Some(n) => format!("<button class=\"btn ghost\" data-t=\"{}\">{} {} →</button>", n.id, wk(n), ttp(n)),
        None => String::new(),
    };
    let h = format!(
        "<div class=\"learn\">\n    <aside class=\"card side\">{side}</aside>\n    <div style=\"min-width:0\">{pick}\n    <article class=\"card reader\">\n      <header><div><div class=\"eyebrow\">{tn} · {code} · {wk}</div><h1 style=\"margin-top:6px\">{title}</h1></div>\n      <button class=\"btn {mc}\" id=\"mark\">{ml}</button></header>\n      {toc}\n      {secs}\n      {worked}{recap}{terms}\n      <section style=\"display:flex;gap:10px;flex-wrap:wrap\"><button class=\"btn ghost\" id=\"drillCards\">Flashcards for this topic</button><button class=\"btn ghost\" id=\"drillQuiz\">Quiz this topic</button></section>\n    </article>\n    <div class=\"pager\">{pp}{pn}</div>\n    </div></div>",
        side = side,
        pick = pick,
        tn = lp(&term.name, &term.zh),
        code = c.mods[&t.module].code,
        wk = wk(t),
        title = tt(t),
        mc = if is_done { "ok" } else { "ghost" },
        ml = if is_done { "✓ Studied" } else { "Mark as studied" },
        toc = toc,
        secs = secs,
        worked = worked,
        recap = recap,
        terms = terms,
        pp = pager_prev,
        pn = pager_next
    );
    let a = app();
    set_html(&a, &h);
    for (i, e) in qsa(&a, ".reader>section>h3").iter().enumerate() {
        set_attr(e, "data-n", &format!("{:02}", i + 1));
    }
    mark_extras(t);
    // wrap every diagram in a numbered figure
    let mut n = 0;
    for s in qsa(&a, ".reader svg.dia") {
        if closest(&s, ".scrdemo").is_some() || closest(&s, "figure").is_some() {
            continue;
        }
        let lab = s.get_attribute("aria-label");
        let f = doc().create_element("figure").unwrap();
        f.set_class_name("fig");
        let _ = s.replace_with_with_node_1(&f);
        let _ = f.append_child(&s);
        if let Some(lab) = lab {
            if !lab.is_empty() {
                n += 1;
                let cap = doc().create_element("figcaption").unwrap();
                set_attr(&cap, "data-tr", "1");
                set_html(&cap, &format!("<b>{} {}</b>{}", if lg == Lang::Zh { "图" } else { "Fig." }, n, esc(&lab)));
                let _ = f.append_child(&cap);
            }
        }
    }
    for el in qsa(&a, ".scrdemo") {
        scr_init(&el);
    }
    annotate_formulas(&t.id);
    notebook::after_render();
}

fn mark_extras(t: &data::Topic) {
    let r = match qs(&app(), ".reader") {
        Some(r) => r,
        None => return,
    };
    let lg = lang();
    let lx = |en: &str, zh: &str| -> String {
        match lg {
            Lang::Zh => zh.to_string(),
            Lang::Both => format!("{} · {}", en, zh),
            Lang::En => en.to_string(),
        }
    };
    let none = vec![];
    let ex = c().xsec.get(&t.id).unwrap_or(&none);
    let secs = qsa(&r, ":scope>section");
    for &i in ex {
        let s = match secs.get(i) {
            Some(s) => s,
            None => continue,
        };
        add_class(s, "xtra");
        if let Some(h3) = qs(s, ":scope>h3") {
            if qs(s, ".xtag").is_none() {
                let d = doc().create_element("div").unwrap();
                d.set_class_name("xtag");
                set_attr(&d, "data-tr", "1");
                set_text(&d, &lx("Extra explanation — goes beyond your uploaded notes", "补充讲解——超出你上传的笔记内容"));
                let _ = h3.after_with_node_1(&d);
            }
        }
        if let Some(b) = qs(&r, &format!(".toc [data-sec-go=\"{}\"]", i)) {
            add_class(&b, "xt");
        }
    }
    let ew = c().xwork.get(&t.id).unwrap_or(&none);
    if t.worked.is_some() {
        let ws = secs.get(t.sections.len()).map(|s| qsa(s, "details.ex>summary")).unwrap_or_default();
        for &i in ew {
            if let Some(s) = ws.get(i) {
                if qs(s, ".xw").is_none() {
                    let b = doc().create_element("span").unwrap();
                    b.set_class_name("xw");
                    set_attr(&b, "data-tr", "1");
                    set_text(&b, &lx("Extra practice", "补充练习"));
                    let _ = s.append_child(&b);
                }
            }
        }
    }
    let hd = qs(&r, ":scope>header");
    let src_only = with(|s| s.src_only);
    if !ex.is_empty() && hd.is_some() && qs(hd.as_ref().unwrap(), "#srcTog").is_none() {
        let b = doc().create_element("button").unwrap();
        b.set_id("srcTog");
        b.set_class_name("btn ghost");
        set_attr(&b, "type", "button");
        set_attr(&b, "data-tr", "1");
        let _ = hd.unwrap().append_child(&b);
        paint_src(&r, &b, ex.len(), src_only);
    } else {
        toggle_class(&r, "srconly", src_only && !ex.is_empty());
    }
}

fn paint_src(r: &Element, b: &Element, n: usize, on: bool) {
    let lg = lang();
    let lx = |en: String, zh: String| -> String {
        match lg {
            Lang::Zh => zh,
            Lang::Both => format!("{} · {}", en, zh),
            Lang::En => en,
        }
    };
    toggle_class(r, "srconly", on);
    set_attr(b, "aria-pressed", if on { "true" } else { "false" });
    set_text(
        b,
        &if on {
            lx(format!("Notes only ✓ ({} extra sections hidden)", n), format!("仅笔记内容 ✓（{} 节补充已隐藏）", n))
        } else {
            lx("Show notes only".into(), "只看笔记内容".into())
        },
    );
}

pub fn toggle_src_only() {
    let an = nav::anchor();
    let on = with(|s| {
        s.src_only = !s.src_only;
        s.src_only
    });
    store_set("srconly", &serde_json::Value::Bool(on));
    let t = with(|s| s.topic.clone());
    let n = c().xsec.get(&t).map(|v| v.len()).unwrap_or(0);
    if let (Some(r), Some(b)) = (qs(&app(), ".reader"), by_id("srcTog")) {
        paint_src(&r, &b, n, on);
    }
    if let Some(a) = an {
        if scroll_y() > 0.0 {
            with(|s| s.nav.lock = true);
            let run = nav::bump_run();
            nav::to_anchor(a, run, false);
        }
    }
}

fn annotate_formulas(topic: &str) {
    let lg = lang();
    let mut count = 0;
    for bx in qsa(&app(), ".reader .formula") {
        let first = match qs(&bx, ":scope>span") {
            Some(f) => f,
            None => continue,
        };
        if js_trim(&first.text_content().unwrap_or_default()).is_empty() {
            continue;
        }
        let k = format!("{}#{}", topic, count);
        count += 1;
        let a = match c().anno.get(&k) {
            Some(a) => a,
            None => continue,
        };
        if !bx.text_content().unwrap_or_default().contains(&a.chk) {
            continue;
        }
        if let Some(n) = bx.next_element_sibling() {
            if has_class(&n, "anno") {
                continue;
            }
        }
        let tx = |en: &str, zh: &str| -> String {
            match lg {
                Lang::En => en.to_string(),
                Lang::Zh => zh.to_string(),
                Lang::Both => format!("{}<span class=\"anz\">{}</span>", en, zh),
            }
        };
        let rows: String = a
            .s
            .iter()
            .map(|r| {
                let u = r.get(3).map(|s| s.as_str()).unwrap_or("");
                format!(
                    "<li><b class=\"asym\">{}</b><span class=\"adesc\">{}</span>{}</li>",
                    r[0],
                    tx(&r[1], &r[2]),
                    if !u.is_empty() && u != "—" { format!("<span class=\"aunit\">{}</span>", u) } else { String::new() }
                )
            })
            .collect();
        let d = doc().create_element("div").unwrap();
        d.set_class_name("anno");
        set_attr(&d, "data-tr", "1");
        set_html(
            &d,
            &format!(
                "<div class=\"anh\">{}</div><ul>{}</ul><p class=\"anr\"><b>{}</b> {}</p>",
                tx("What each symbol means", "各符号的含义"),
                rows,
                tx("Reading it:", "理解要点："),
                tx(&a.r[0], &a.r[1])
            ),
        );
        let _ = bx.after_with_node_1(&d);
    }
}

// ---------- SCR firing-angle explorer ----------
fn scr_init(el: &Element) {
    let h = format!(
        "<div class=\"scrc\"><div class=\"row\">\n   <div class=\"field\"><label for=\"scr-t\">{}</label><select id=\"scr-t\"><option value=\"hw\">{}</option><option value=\"half\" selected>{}</option><option value=\"full\">{}</option></select></div>\n   <div class=\"field\"><label for=\"scr-v\">{}</label><input id=\"scr-v\" type=\"number\" step=\"any\" inputmode=\"decimal\" value=\"230\"></div>\n   <div class=\"field\"><label for=\"scr-n\">{}</label><input id=\"scr-n\" type=\"number\" step=\"any\" inputmode=\"decimal\" value=\"1500\"></div></div>\n   <label for=\"scr-a\" class=\"scrlab\">{} <b id=\"scr-av\">60°</b></label>\n   <input id=\"scr-a\" class=\"scrrange\" type=\"range\" min=\"0\" max=\"180\" step=\"1\" value=\"60\">\n   <svg id=\"scr-svg\" class=\"dia\" viewBox=\"0 0 560 210\" role=\"img\" aria-label=\"{}\"></svg>\n   <p class=\"nbhint\">{}</p>\n   <dl class=\"out\" id=\"scr-out\"></dl>\n   <p class=\"nbhint\">{}</p></div>",
        lp("Converter", "变流器"),
        lp("Half-wave (1 SCR)", "半波（1 个可控硅）"),
        lp("Half-controlled bridge", "半控桥"),
        lp("Fully controlled bridge", "全控桥"),
        lp("Supply (V rms)", "电源（V 有效值）"),
        lp("Speed at α = 0° (rpm)", "α = 0° 时转速 (rpm)"),
        lp("Firing angle α", "触发角 α"),
        lp("Output voltage waveform", "输出电压波形"),
        lp("Dashed = AC supply · orange = voltage reaching the motor · blue arrows = gate pulses · blue line = average voltage (Vdc)", "虚线 = 交流电源 · 橙色 = 送到电机的电压 · 蓝色箭头 = 门极脉冲 · 蓝线 = 平均电压 (Vdc)"),
        lp("Drag the slider: firing later cuts a smaller slice of each wave, so the average voltage and speed fall. Speed estimate ignores the IaRa drop and assumes a constant field.", "拖动滑块：触发越晚，每个波形导通的部分越小，平均电压和转速随之下降。转速估算忽略 IaRa 压降，并假设励磁不变。")
    );
    set_html(el, &h);
    scr_draw(el);
}

pub fn scr_draw(el: &Element) {
    use std::f64::consts::PI;
    let g = |id: &str| qs(el, &format!("#{}", id)).unwrap();
    let a = js_number(&input_value(&g("scr-a")));
    let t = input_value(&g("scr-t"));
    let v = parse_float(&input_value(&g("scr-v")));
    let n0 = parse_float(&input_value(&g("scr-n")));
    set_text(&g("scr-av"), &format!("{}°", num_str(a)));
    let ar = a * PI / 180.0;
    let (x0, w, y0, amp) = (20.0, 520.0, 105.0, 78.0);
    let to_x = |th: f64| x0 + th / (2.0 * PI) * w;
    let to_y = |v: f64| y0 - v * amp;
    let out = |th: f64| -> f64 {
        let sn = th.sin();
        if t == "hw" {
            return if th >= ar && th <= PI { sn } else { 0.0 };
        }
        let ph = th % PI;
        if t == "half" {
            return if ph >= ar { sn.abs() } else { 0.0 };
        }
        let d = ((th - ar) % (2.0 * PI) + 2.0 * PI) % (2.0 * PI);
        if d < PI { sn } else { -sn }
    };
    let mut inp = String::new();
    let mut o = String::new();
    let mut fill = format!("M{} {}", num_str(to_x(0.0)), num_str(y0));
    for i in 0..=360 {
        let th = i as f64 / 360.0 * 2.0 * PI;
        let m = if i > 0 { "L" } else { "M" };
        inp.push_str(&format!("{}{} {}", m, to_fixed(to_x(th), 1), to_fixed(to_y(th.sin()), 1)));
        let vv = out(th);
        o.push_str(&format!("{}{} {}", m, to_fixed(to_x(th), 1), to_fixed(to_y(vv), 1)));
        fill.push_str(&format!("L{} {}", to_fixed(to_x(th), 1), to_fixed(to_y(vv), 1)));
    }
    fill.push_str(&format!("L{} {}Z", num_str(to_x(2.0 * PI)), num_str(y0)));
    let pulses: Vec<f64> = if t == "hw" { vec![ar] } else { vec![ar, ar + PI] }.into_iter().filter(|_| a < 180.0).collect();
    let vm = std::f64::consts::SQRT_2 * v;
    let vdc = if t == "hw" {
        vm * (1.0 + ar.cos()) / (2.0 * PI)
    } else if t == "half" {
        vm * (1.0 + ar.cos()) / PI
    } else {
        2.0 * vm * ar.cos() / PI
    };
    let vmax = if t == "hw" { vm / PI } else { 2.0 * vm / PI };
    let avg_y = to_y(vdc / vm);
    let pul: String = pulses
        .iter()
        .map(|&p| {
            format!(
                "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"var(--bolt)\" stroke-width=\"2\"/><polygon points=\"{},{} {},{} {},{}\" fill=\"var(--bolt)\"/>",
                num_str(y0 + 8.0),
                num_str(y0 + 92.0),
                to_fixed(to_x(p) - 5.0, 1),
                num_str(y0 + 92.0),
                to_fixed(to_x(p) + 5.0, 1),
                num_str(y0 + 92.0),
                to_fixed(to_x(p), 1),
                num_str(y0 + 101.0),
                x = to_fixed(to_x(p), 1)
            )
        })
        .collect();
    let svg = format!(
        "<line class=\"ln\" x1=\"{x0}\" y1=\"{y0}\" x2=\"{xe}\" y2=\"{y0}\" stroke-dasharray=\"3 4\"/>\n     <path d=\"{inp}\" fill=\"none\" stroke=\"var(--muted)\" stroke-width=\"1.5\" stroke-dasharray=\"5 4\" opacity=\".7\"/>\n     <path d=\"{fill}\" fill=\"var(--accent-soft)\" stroke=\"none\"/><path d=\"{o}\" fill=\"none\" stroke=\"var(--accent)\" stroke-width=\"2.5\"/>\n     <line x1=\"{x0}\" y1=\"{ay}\" x2=\"{xe}\" y2=\"{ay}\" stroke=\"var(--bolt)\" stroke-width=\"2\"/>\n     <text x=\"{tx}\" y=\"{ty}\" text-anchor=\"end\" fill=\"var(--bolt)\">V<tspan dy=\"3\" style=\"font-size:9px\">dc</tspan></text>\n     {pul}",
        x0 = num_str(x0),
        y0 = num_str(y0),
        xe = num_str(x0 + w),
        inp = inp,
        fill = fill,
        o = o,
        ay = to_fixed(avg_y, 1),
        tx = num_str(x0 + w - 4.0),
        ty = to_fixed(avg_y - 6.0, 1),
        pul = pul
    );
    set_html(&g("scr-svg"), &svg);
    let ok = v.is_finite() && n0.is_finite();
    let spd = if vdc > 0.5 {
        format!("{} rpm", fmt(n0 * vdc / vmax, 0))
    } else if t == "full" && vdc < -0.5 {
        lp("— inverting: energy returned to supply (regenerative braking)", "— 逆变：能量回馈电网（再生制动）")
    } else {
        lp("0 rpm (stopped)", "0 rpm（停止）")
    };
    let out_h = if ok {
        format!(
            "<dt>{}</dt><dd>{} V</dd><dt>{}</dt><dd>{} V</dd><dt>{}</dt><dd>{} %</dd><dt>{}</dt><dd>{}</dd>",
            lp("Peak voltage Vm", "峰值电压 Vm"),
            fmt(vm, 1),
            lp("Average output Vdc", "平均输出 Vdc"),
            fmt(vdc, 1),
            lp("Percent of maximum", "占最大值的百分比"),
            fmt(vdc / vmax * 100.0, 0),
            lp("Approx. motor speed", "电机转速（近似）"),
            spd
        )
    } else {
        format!("<div class=\"err\">{}</div>", lp("Enter a number in every field.", "请在每个输入框中填入数字。"))
    };
    set_html(&g("scr-out"), &out_h);
}

// ---------- events ----------
pub fn on_click(t: &Element) -> bool {
    let a = app();
    if let Some(b) = closest(t, "#app [data-t]") {
        set_topic(&attr(&b, "data-t"), true);
        render();
        return true;
    }
    if closest(t, "#mark").is_some() {
        let id = {
            let topic = with(|s| s.topic.clone());
            c().topic(&topic).unwrap_or(&c().topics[0]).id.clone()
        };
        with(|s| {
            if s.done.contains(&id) {
                s.done.shift_remove(&id);
            } else {
                s.done.insert(id.clone());
            }
            store_set("done", &serde_json::Value::Array(s.done.iter().map(|x| serde_json::Value::String(x.clone())).collect()));
        });
        render();
        return true;
    }
    if let Some(b) = closest(t, "#app .blank") {
        set_text(&b, &attr(&b, "data-a"));
        add_class(&b, "shown");
        return true;
    }
    if closest(t, "#drillCards").is_some() {
        let id = cur_topic_id();
        with(|s| s.card.filter = id);
        go("cards", GoOpts::default());
        return true;
    }
    if closest(t, "#drillQuiz").is_some() {
        let id = cur_topic_id();
        with(|s| s.quiz.filter = id);
        quiz::start();
        go("quiz", GoOpts::default());
        return true;
    }
    if closest(t, "#srcTog").is_some() {
        toggle_src_only();
        return true;
    }
    let _ = a;
    false
}
fn cur_topic_id() -> String {
    let topic = with(|s| s.topic.clone());
    c().topic(&topic).unwrap_or(&c().topics[0]).id.clone()
}
pub fn on_change(t: &Element) {
    if t.id() == "pick" {
        set_topic(&input_value(t), true);
        render();
    }
}
pub fn on_input(t: &Element) {
    if let Some(sd) = closest(t, ".scrdemo") {
        scr_draw(&sd);
    }
}
pub fn on_toggle(t: &Element) {
    if t.matches("details.smod").unwrap_or(false) && app().contains(Some(t)) {
        let o: Vec<String> = qsa(&app(), "details.smod[open]").iter().map(|x| attr(x, "data-m")).collect();
        ls_set("sh_sideopen", &serde_json::to_string(&o).unwrap());
    }
}
