//! Overview tab.
use crate::util::*;
use crate::*;

pub fn ring(pct: f64, label: &str) -> String {
    let r = 27.0;
    let circ = 2.0 * std::f64::consts::PI * r;
    format!(
        "<svg class=\"ring\" viewBox=\"0 0 64 64\" aria-label=\"{} {}%\"><circle class=\"bgc\" cx=\"32\" cy=\"32\" r=\"{}\"/><circle class=\"fgc\" cx=\"32\" cy=\"32\" r=\"{}\" stroke-dasharray=\"{}\" stroke-dashoffset=\"{}\" transform=\"rotate(-90 32 32)\"/><text x=\"32\" y=\"37\" text-anchor=\"middle\">{}%</text></svg>",
        label,
        num_str(pct),
        num_str(r),
        num_str(r),
        num_str(circ),
        num_str(circ * (1.0 - pct / 100.0)),
        num_str(pct)
    )
}

pub fn round(x: f64) -> f64 {
    // Math.round
    (x + 0.5).floor()
}

pub fn render() {
    let c = c();
    let (done, known_n, best) = with(|s| (s.done.clone(), s.known.len(), s.best));
    let cards_n = c.cards.len();
    let card_pct = round(known_n as f64 / cards_n as f64 * 100.0);
    let mod_topics = |m: &str| c.topics.iter().filter(|t| t.module == m).collect::<Vec<_>>();
    let mod_card = |m: &String| -> String {
        let ts = mod_topics(m);
        let d = ts.iter().filter(|t| done.contains(&t.id)).count();
        let p = round(d as f64 / ts.len() as f64 * 100.0);
        let nx = ts.iter().find(|t| !done.contains(&t.id)).copied().unwrap_or(ts[0]);
        let md = &c.mods[m.as_str()];
        let btn = if d == ts.len() {
            lp("Review", "复习")
        } else if d > 0 {
            lp("Continue", "继续")
        } else {
            lp("Start", "开始")
        };
        let list: String = ts
            .iter()
            .map(|t| {
                format!(
                    "<li><button data-topic=\"{}\"><span class=\"done-dot {}\"></span><span class=\"wk\">{}</span><span>{}</span></button></li>",
                    t.id,
                    if done.contains(&t.id) { "on" } else { "" },
                    wk(t),
                    tt(t)
                )
            })
            .collect();
        format!(
            "<div class=\"card mod modc\"><div class=\"eyebrow\">{code}</div><h3 class=\"modt\">{name}</h3>\n    <div class=\"pbar\" role=\"progressbar\" aria-valuenow=\"{p}\" aria-valuemin=\"0\" aria-valuemax=\"100\"><span style=\"width:{p}%\"></span></div>\n    <div class=\"modmeta\">{meta}</div>\n    <button class=\"btn ghost sm\" data-topic=\"{nx}\">{btn} · {nwk}</button>\n    <details class=\"modlist\"><summary>{all}</summary><ol>{list}</ol></details></div>",
            code = md.code,
            name = mn(m),
            p = num_str(p),
            meta = l(&format!("{} of {} studied", d, ts.len()), &format!("已学 {} / {}", d, ts.len())),
            nx = nx.id,
            btn = btn,
            nwk = wk(nx),
            all = l(&format!("All {} topics", ts.len()), &format!("全部 {} 个主题", ts.len())),
            list = list
        )
    };
    let progs: String = c
        .programs
        .iter()
        .map(|p| {
            let badge = match p.status.as_str() {
                "current" => lp("Current", "进行中"),
                "extra" => lp("Extra", "拓展"),
                _ => lp("Next · AY2027", "下一阶段 · 2027 学年"),
            };
            let body = if !p.terms.is_empty() {
                p.terms
                    .iter()
                    .map(|t| {
                        format!(
                            "<div class=\"term\"><h3 class=\"termh\">{}</h3><div class=\"modgrid\">{}</div></div>",
                            l(&t.name, &t.zh),
                            t.mods.iter().map(|m| mod_card(m)).collect::<String>()
                        )
                    })
                    .collect::<String>()
            } else {
                format!("<p class=\"muted\">{}</p>", tl(p.note.as_deref().unwrap_or(""), p.znote.as_deref()))
            };
            format!(
                "<section class=\"card prog {st}\">\n    <div class=\"proghead\"><div><div class=\"eyebrow\">{inst}</div><h2>{name}</h2></div><span class=\"badge {st}\">{badge}</span></div>\n    {body}\n  </section>",
                st = p.status,
                inst = l(&p.inst, &p.zinst),
                name = l(&p.name, &p.zh),
                badge = badge,
                body = body
            )
        })
        .collect();
    let n_t = c.topics.len();
    let n_m = c.mod_order.len();
    let n_q = c.quiz.len();
    let intro = l(
        &format!("{} topics across {} modules, organised by programme and semester. Each topic has condensed notes with diagrams and every worked example solved, then {} flashcards and {} quiz questions to drill. The calculators check your answers, the formula sheet explains every symbol, and the PLC Lab runs your own ladder programs.", n_t, n_m, cards_n, n_q),
        &format!("共 {} 个模块、{} 个主题，按课程和学期整理。每个主题都有带图解的精简笔记和全部例题解答，另有 {} 张抽认卡和 {} 道测验题。计算器可核对答案，公式表解释每个符号，PLC 实验室可运行你自己的梯形图程序。", n_m, n_t, cards_n, n_q),
    );
    let stats: String = c
        .programs
        .iter()
        .filter(|p| p.status == "current")
        .flat_map(|p| p.terms.iter())
        .map(|tm| {
            let ts: Vec<_> = c.topics.iter().filter(|t| tm.mods.contains(&t.module)).collect();
            let d = ts.iter().filter(|t| done.contains(&t.id)).count();
            format!(
                "<div class=\"card stat\">{}<div><div class=\"lbl\">{}</div><div class=\"big\">{} / {}</div></div></div>",
                ring(round(d as f64 / ts.len() as f64 * 100.0), &tm.name),
                l(&format!("{} topics studied", tm.name), &format!("{}已学主题", tm.zh)),
                d,
                ts.len()
            )
        })
        .collect();
    let best_pct = best.map(|(s, n)| round(s / n * 100.0)).unwrap_or(0.0);
    let best_txt = best.map(|(s, n)| format!("{} / {}", num_str(s), num_str(n))).unwrap_or_else(|| "—".into());
    let h = format!(
        "\n  <div class=\"hero\">\n    <div class=\"card intro\">\n      <div class=\"eyebrow\">{eb}</div>\n      <h1 style=\"margin:8px 0 10px\">{h1}</h1>\n      <p class=\"muted\">{intro}</p>\n      <div style=\"display:flex;gap:10px;flex-wrap:wrap;margin-top:14px\">\n        <button class=\"btn\" data-go=\"learn\">Continue notes</button>\n        <button class=\"btn ghost\" data-go=\"quiz\">Take a quiz</button>\n        <button class=\"btn ghost\" data-go=\"calc\">Calculators</button>{dl}\n      </div>\n    </div>\n    <div class=\"stats\">\n      {stats}\n      <div class=\"card stat\">{r1}<div><div class=\"lbl\">{l1}</div><div class=\"big\">{k} / {cn}</div></div></div>\n      <div class=\"card stat\">{r2}<div><div class=\"lbl\">{l2}</div><div class=\"big\">{bt}</div></div></div>\n    </div>\n  </div>\n  {progs}",
        eb = l("Mechatronics learning hub", "机电一体化学习中心"),
        h1 = l("Study every note in one place", "所有笔记，一处学习"),
        intro = intro,
        stats = stats,
        r1 = ring(card_pct, "Cards"),
        l1 = l("Flashcards known", "已掌握抽认卡"),
        k = known_n,
        cn = cards_n,
        r2 = ring(best_pct, "Quiz"),
        l2 = l("Best quiz score", "最佳测验成绩"),
        bt = best_txt,
        progs = progs,
        dl = if is_desktop() { "" } else { "\n        <a class=\"btn ghost\" id=\"dlApp\" href=\"https://weiqianw.github.io/studyhub/download/\" target=\"_blank\" rel=\"noopener\">Download Windows app</a>" }
    );
    set_html(&app(), &h);
}

pub fn on_click(t: &web_sys::Element) -> bool {
    if let Some(b) = closest(t, "#app [data-topic]") {
        let id = attr(&b, "data-topic");
        set_topic(&id, true);
        go("learn", GoOpts::default());
        return true;
    }
    if let Some(b) = closest(t, "#app [data-go]") {
        go(&attr(&b, "data-go"), GoOpts::default());
        return true;
    }
    false
}

/// True inside the Windows app (it sets `window.SH_DESKTOP`), so the download button is hidden there.
fn is_desktop() -> bool {
    js_sys::Reflect::get(&win(), &wasm_bindgen::JsValue::from_str("SH_DESKTOP")).map(|v| v.is_truthy()).unwrap_or(false)
}
