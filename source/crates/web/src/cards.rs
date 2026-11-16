//! Flashcards tab.
use crate::data::Card;
use crate::util::*;
use crate::*;
use web_sys::Element;

pub struct CardSt {
    pub filter: String,
    pub order: Vec<String>,
    pub i: usize,
    pub flipped: bool,
    pub only_unknown: bool,
}
impl Default for CardSt {
    fn default() -> Self {
        CardSt { filter: "all".into(), order: vec![], i: 0, flipped: false, only_unknown: false }
    }
}

pub fn shuffle<T>(a: &mut [T]) {
    if a.is_empty() {
        return;
    }
    for i in (1..a.len()).rev() {
        let j = (random() * (i as f64 + 1.0)).floor() as usize;
        a.swap(i, j);
    }
}

/// Does card/quiz row with topic `topic` pass `filter` ('all', a module id, or a topic id)?
pub fn passes(filter: &str, topic: &str) -> bool {
    if filter == "all" {
        return true;
    }
    if c().mods.contains_key(filter) {
        if let Some(t) = c().topic(topic) {
            if t.module == filter {
                return true;
            }
        }
    }
    topic == filter
}

fn filtered() -> Vec<&'static Card> {
    let (f, ou) = with(|s| (s.card.filter.clone(), s.card.only_unknown));
    let mut l: Vec<&Card> = c().cards.iter().filter(|x| passes(&f, &x.topic)).collect();
    if ou {
        l = with(|s| l.into_iter().filter(|x| !s.known.contains(&x.id)).collect());
    }
    l
}

pub fn topic_options(sel: &str) -> String {
    let c = c();
    let zh = lang() == Lang::Zh;
    let s = |v: &str| if sel == v { "selected" } else { "" };
    let mut h = format!("<option value=\"all\" {}>{}</option>", s("all"), if zh { "全部主题" } else { "All topics" });
    for m in &c.mod_order {
        let md = &c.mods[m];
        h.push_str(&format!(
            "<optgroup label=\"{} {}\"><option value=\"{}\" {}>{}</option>",
            md.code,
            lp(&md.name, &md.zh),
            m,
            s(m),
            if zh { format!("全部 {} {}", md.code, md.zh) } else { format!("All {} {}", md.code, md.name) }
        ));
        for t in c.topics.iter().filter(|t| t.module == *m) {
            h.push_str(&format!("<option value=\"{}\" {}>{} · {}</option>", t.id, s(&t.id), wk(t), ttp(t)));
        }
        h.push_str("</optgroup>");
    }
    h
}

pub fn render() {
    let list = filtered();
    with(|s| {
        let o = &s.card.order;
        let bad = o.is_empty() || o.iter().any(|id| !list.iter().any(|c| &c.id == id)) || o.len() != list.len();
        if bad {
            s.card.order = list.iter().map(|c| c.id.clone()).collect();
            s.card.i = 0;
        }
    });
    let (order, i, flipped, filter, ou) = with(|s| (s.card.order.clone(), s.card.i, s.card.flipped, s.card.filter.clone(), s.card.only_unknown));
    let card = order.get(i).and_then(|id| c().cards.iter().find(|x| &x.id == id));
    let known_in = with(|s| list.iter().filter(|x| s.known.contains(&x.id)).count());
    let lg = lang();
    let (mut cq, mut ca) = (String::new(), String::new());
    if let Some(cd) = card {
        if cd.term {
            let en = cd.en.as_deref().unwrap_or("");
            let zh = cd.zh.as_deref().unwrap_or("");
            if lg == Lang::Zh {
                cq = format!("术语：<b>{}</b>", zh);
                ca = en.to_string();
            } else {
                cq = tl(&format!("Term: <b>{}</b>", en), if lg == Lang::Both { Some("术语") } else { None });
                ca = format!("<span style=\"font-family:var(--f-zh)\">{}</span>", zh);
            }
        } else {
            cq = tl(cd.q.as_deref().unwrap_or(""), cd.zq.as_deref());
            ca = tl(cd.a.as_deref().unwrap_or(""), cd.za.as_deref());
        }
    }
    let body = match card {
        Some(cd) => {
            let t = c().topic(&cd.topic).unwrap();
            let known = with(|s| s.known.contains(&cd.id));
            format!(
                "<div class=\"flash {fl}\" id=\"fl\"><div class=\"inner\" tabindex=\"0\" role=\"button\" aria-label=\"Flip card\">\n    <div class=\"face\"><div class=\"meta\"><span>{wk} · {tt}</span><span>{n} / {len}</span></div><div class=\"q\">{cq}</div><div class=\"muted\" style=\"font-size:13px\">Tap or press Space to flip</div></div>\n    <div class=\"face back\"><div class=\"meta\"><span>Answer</span><span>{kn}</span></div><div class=\"a\">{ca}</div></div>\n  </div></div>\n  <div class=\"cardctl\"><button class=\"btn ghost\" id=\"prev\">← Prev</button><button class=\"btn warn\" id=\"again\">Still learning</button><button class=\"btn ok\" id=\"got\">I know this</button><button class=\"btn ghost\" id=\"next\">Next →</button></div>",
                fl = if flipped { "flipped" } else { "" },
                wk = wk(t),
                tt = ttp(t),
                n = i + 1,
                len = list.len(),
                cq = cq,
                kn = if known { "✓ known" } else { "" },
                ca = ca
            )
        }
        None => "<div class=\"card\" style=\"padding:30px;text-align:center;max-width:720px;margin:0 auto\"><h3>All cards in this set are marked known</h3><p class=\"muted\">Turn off the filter or reset to go again.</p><button class=\"btn\" id=\"resetK\">Reset known cards for this set</button></div>".to_string(),
    };
    let w = if !list.is_empty() { known_in as f64 / list.len() as f64 * 100.0 } else { 100.0 };
    let h = format!(
        "<div class=\"toolbar\"><h2 style=\"margin-right:auto\">Flashcards</h2>\n    <select class=\"sel\" id=\"cf\" aria-label=\"Topic filter\">{opts}</select>\n    <button class=\"chipbtn\" id=\"unk\" aria-pressed=\"{ou}\">Only cards I'm still learning</button>\n    <button class=\"chipbtn\" id=\"shuf\">Shuffle</button></div>\n  {body}\n  <div class=\"bar\" aria-label=\"Known in this set\"><i style=\"width:{w}%\"></i></div>\n  <p class=\"muted\" style=\"text-align:center;font-size:13px\">{k} of {n} known in this set</p>",
        opts = topic_options(&filter),
        ou = ou,
        body = body,
        w = num_str(w),
        k = known_in,
        n = list.len()
    );
    set_html(&app(), &h);
}

fn save_known() {
    with(|s| store_set("known", &serde_json::Value::Array(s.known.iter().map(|x| serde_json::Value::String(x.clone())).collect())));
}
fn mv(d: i64) {
    with(|s| {
        let n = s.card.order.len() as i64;
        if n > 0 {
            s.card.i = ((s.card.i as i64 + d + n) % n) as usize;
        }
        s.card.flipped = false;
    });
    render();
}
fn flip() {
    with(|s| s.card.flipped = !s.card.flipped);
    if let Some(f) = by_id("fl") {
        let _ = f.class_list().toggle("flipped");
    }
}

pub fn on_click(t: &Element) -> bool {
    if closest(t, "#unk").is_some() {
        with(|s| {
            s.card.only_unknown = !s.card.only_unknown;
            s.card.order.clear();
            s.card.flipped = false;
        });
        render();
        return true;
    }
    if closest(t, "#shuf").is_some() {
        let mut ids: Vec<String> = filtered().iter().map(|x| x.id.clone()).collect();
        shuffle(&mut ids);
        with(|s| {
            s.card.order = ids;
            s.card.i = 0;
            s.card.flipped = false;
        });
        render();
        return true;
    }
    if closest(t, "#resetK").is_some() {
        let f = with(|s| s.card.filter.clone());
        // the original's reset rule: whole set, the exact topic, or (for 3-letter module ids) same first letter
        let ids: Vec<String> = c()
            .cards
            .iter()
            .filter(|x| {
                f == "all" || x.topic == f || (x.topic.chars().next() == f.chars().next() && f.encode_utf16().count() == 3)
            })
            .map(|x| x.id.clone())
            .collect();
        with(|s| {
            for id in ids {
                s.known.shift_remove(&id);
            }
        });
        save_known();
        with(|s| {
            s.card.only_unknown = false;
            s.card.order.clear();
        });
        render();
        return true;
    }
    if closest(t, "#app .inner").is_some() {
        flip();
        return true;
    }
    if closest(t, "#prev").is_some() {
        mv(-1);
        return true;
    }
    if closest(t, "#next").is_some() {
        mv(1);
        return true;
    }
    if closest(t, "#got").is_some() {
        let id = with(|s| s.card.order.get(s.card.i).cloned());
        if let Some(id) = id {
            with(|s| {
                s.known.insert(id);
            });
            save_known();
            if with(|s| s.card.only_unknown) {
                with(|s| {
                    let i = s.card.i;
                    s.card.order.remove(i);
                    if s.card.i >= s.card.order.len() {
                        s.card.i = 0;
                    }
                    s.card.flipped = false;
                });
                render();
            } else {
                mv(1);
            }
        }
        return true;
    }
    if closest(t, "#again").is_some() {
        let id = with(|s| s.card.order.get(s.card.i).cloned());
        if let Some(id) = id {
            with(|s| {
                s.known.shift_remove(&id);
            });
            save_known();
            mv(1);
        }
        return true;
    }
    false
}
pub fn on_change(t: &Element) {
    if t.id() == "cf" {
        let v = input_value(t);
        with(|s| {
            s.card.filter = v;
            s.card.order.clear();
            s.card.flipped = false;
        });
        render();
    }
}
pub fn on_keydown(e: &web_sys::KeyboardEvent, t: &Element) {
    let k = e.key();
    if closest(t, "#app .inner").is_some() && (k == " " || k == "Enter") {
        e.prevent_default();
        flip();
        return;
    }
    if t.tag_name() == "SELECT" {
        return;
    }
    if k == "ArrowRight" {
        if by_id("next").is_some() {
            mv(1);
        }
    }
    if k == "ArrowLeft" {
        if by_id("prev").is_some() {
            mv(-1);
        }
    }
}
