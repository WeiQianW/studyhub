//! Quiz tab.
use crate::cards::{passes, shuffle, topic_options};
use crate::data::{va, vs};
use crate::util::*;
use crate::*;
use web_sys::Element;

#[derive(Clone)]
pub struct Q {
    t: String,
    q: String,
    zq: Option<String>,
    opts: Vec<String>,
    zopts: Vec<Option<String>>,
    ans: usize,
    ex: String,
    zex: Option<String>,
}

pub struct QuizSt {
    pub filter: String,
    pub n: usize,
    qs: Vec<Q>,
    i: usize,
    answers: Vec<Option<usize>>,
    done: bool,
}
impl Default for QuizSt {
    fn default() -> Self {
        QuizSt { filter: "all".into(), n: 10, qs: vec![], i: 0, answers: vec![], done: false }
    }
}

pub fn start() {
    let c = c();
    let (f, n) = with(|s| (s.quiz.filter.clone(), s.quiz.n));
    let mut pool: Vec<usize> = (0..c.quiz.len()).filter(|&i| passes(&f, vs(&c.quiz[i], 0).unwrap_or(""))).collect();
    shuffle(&mut pool);
    pool.truncate(n);
    let qs: Vec<Q> = pool
        .iter()
        .map(|&qi| {
            let q = &c.quiz[qi];
            let z = c.zq.get(qi);
            let opts: Vec<String> = va(q, 2).map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default();
            let mut idx: Vec<usize> = (0..opts.len().min(4)).collect();
            shuffle(&mut idx);
            let zo = z.and_then(|z| va(z, 1));
            let ans = q.get(3).and_then(|x| x.as_u64()).unwrap_or(0) as usize;
            Q {
                t: vs(q, 0).unwrap_or("").into(),
                q: vs(q, 1).unwrap_or("").into(),
                zq: z.and_then(|z| vs(z, 0)).map(String::from),
                opts: idx.iter().map(|&i| opts[i].clone()).collect(),
                zopts: idx.iter().map(|&i| zo.and_then(|a| a.get(i)).and_then(|x| x.as_str()).map(String::from)).collect(),
                ans: idx.iter().position(|&i| i == ans).unwrap_or(usize::MAX),
                ex: vs(q, 4).unwrap_or("").into(),
                zex: z.and_then(|z| vs(z, 2)).map(String::from),
            }
        })
        .collect();
    with(|s| {
        s.quiz.qs = qs;
        s.quiz.i = 0;
        s.quiz.answers.clear();
        s.quiz.done = false;
    });
}

fn score() -> (usize, usize) {
    with(|s| {
        let sc = s.quiz.qs.iter().enumerate().filter(|(i, q)| s.quiz.answers.get(*i).copied().flatten() == Some(q.ans)).count();
        (sc, s.quiz.qs.len())
    })
}

pub fn render() {
    if with(|s| s.quiz.qs.is_empty()) {
        start();
    }
    let (filter, n) = with(|s| (s.quiz.filter.clone(), s.quiz.n));
    let head = format!(
        "<div class=\"toolbar\"><h2 style=\"margin-right:auto\">Quiz</h2>\n    <select class=\"sel\" id=\"qf\" aria-label=\"Quiz topic\">{}</select>\n    <select class=\"sel\" id=\"qn\" aria-label=\"Number of questions\">{}</select>\n    <button class=\"btn\" id=\"qnew\">New quiz</button></div>",
        topic_options(&filter),
        [5, 10, 20, 40].iter().map(|&k| format!("<option {} value=\"{}\">{} questions</option>", if n == k { "selected" } else { "" }, k, k)).collect::<String>()
    );
    let (done, qs, i, answers) = with(|s| (s.quiz.done, s.quiz.qs.clone(), s.quiz.i, s.quiz.answers.clone()));
    let ans_at = |i: usize| answers.get(i).copied().flatten();
    let body = if done {
        let (s, n) = score();
        let ratio = s as f64 / n as f64;
        let wrong: Vec<&Q> = qs.iter().enumerate().filter(|(i, q)| ans_at(*i) != Some(q.ans)).map(|(_, q)| q).collect();
        let review = if !wrong.is_empty() {
            format!(
                "<h3>Review</h3><ul class=\"review\">{}</ul>",
                wrong
                    .iter()
                    .map(|w| {
                        let ok = w.opts.get(w.ans).cloned().unwrap_or_else(|| "undefined".into());
                        let zok = w.zopts.get(w.ans).cloned().flatten();
                        format!(
                            "<li><b>{}</b><br>{}: {} — <span class=\"muted\">{}</span> <button class=\"chipbtn\" data-t=\"{}\">{} notes</button></li>",
                            tl(&w.q, w.zq.as_deref()),
                            l("Answer", "答案"),
                            tl(&ok, zok.as_deref()),
                            tl(&w.ex, w.zex.as_deref()),
                            w.t,
                            c().topic(&w.t).map(|t| t.wk.clone()).unwrap_or_default()
                        )
                    })
                    .collect::<String>()
            )
        } else {
            String::new()
        };
        format!(
            "<div class=\"card quiz\"><div class=\"score\"><div class=\"eyebrow\">Result</div><div class=\"num\" style=\"color:{}\">{}/{}</div><p class=\"muted\">{}</p></div>\n    {}\n    <div style=\"text-align:center;margin-top:16px\"><button class=\"btn\" id=\"again\">Try another set</button></div></div>",
            if ratio >= 0.7 { "var(--ok)" } else { "var(--warn)" },
            s,
            n,
            if s == n {
                "Perfect score."
            } else if ratio >= 0.7 {
                "Solid — review the misses below."
            } else {
                "Worth another pass through the notes for these topics."
            },
            review
        )
    } else {
        match qs.get(i) {
            Some(q) => {
                let a = ans_at(i);
                let answered = a.is_some();
                let t = c().topic(&q.t).unwrap();
                let opts: String = q
                    .opts
                    .iter()
                    .enumerate()
                    .map(|(k, o)| {
                        format!(
                            "<button class=\"opt {} {}\" data-i=\"{}\" {}><span class=\"k\">{}</span><span>{}</span></button>",
                            if answered && k == q.ans { "right" } else { "" },
                            if answered && Some(k) == a && a != Some(q.ans) { "wrong" } else { "" },
                            k,
                            if answered { "disabled" } else { "" },
                            &"ABCD"[k..k + 1],
                            tl(o, q.zopts.get(k).cloned().flatten().as_deref())
                        )
                    })
                    .collect();
                let after = if answered {
                    format!(
                        "<div class=\"explain\"><b>{}</b> {}</div><div style=\"text-align:right;margin-top:14px\"><button class=\"btn\" id=\"qnext\">{}</button></div>",
                        if a == Some(q.ans) { "Correct." } else { "Not quite." },
                        tl(&q.ex, q.zex.as_deref()),
                        if i + 1 < qs.len() { "Next question →" } else { "See result" }
                    )
                } else {
                    String::new()
                };
                format!(
                    "<div class=\"card quiz\"><div class=\"qhead\"><span>Question {} of {}</span><span>{} · {}</span></div>\n    <div class=\"qtext\">{}</div>\n    <div class=\"opts\">{}</div>\n    {}\n    </div>",
                    i + 1,
                    qs.len(),
                    wk(t),
                    ttp(t),
                    tl(&q.q, q.zq.as_deref()),
                    opts,
                    after
                )
            }
            None => "<div class=\"card quiz\">No questions for this filter.</div>".into(),
        }
    };
    set_html(&app(), &(head + &body));
}

pub fn on_click(t: &Element) -> bool {
    if closest(t, "#app #again").is_some() {
        start();
        render();
        return true;
    }
    if let Some(b) = closest(t, "#app [data-t]") {
        set_topic(&attr(&b, "data-t"), false);
        go("learn", GoOpts::default());
        return true;
    }
    if let Some(b) = closest(t, "#app .opt") {
        if b.has_attribute("disabled") {
            return true;
        }
        let k = parse_int(&attr(&b, "data-i")) as usize;
        with(|s| {
            let i = s.quiz.i;
            if s.quiz.answers.len() <= i {
                s.quiz.answers.resize(i + 1, None);
            }
            s.quiz.answers[i] = Some(k);
        });
        render();
        return true;
    }
    if closest(t, "#qnext").is_some() {
        let more = with(|s| s.quiz.i + 1 < s.quiz.qs.len());
        if more {
            with(|s| s.quiz.i += 1);
        } else {
            with(|s| s.quiz.done = true);
            let (sc, n) = score();
            let (sc, n) = (sc as f64, n as f64);
            let upd = with(|s| {
                n >= 5.0
                    && match s.best {
                        None => true,
                        Some((bs, bn)) => sc / n > bs / bn || (sc / n == bs / bn && n > bn),
                    }
            });
            if upd {
                with(|s| s.best = Some((sc, n)));
                store_set("best", &serde_json::json!({"s": sc as i64, "n": n as i64}));
            }
        }
        render();
        return true;
    }
    if closest(t, "#qnew").is_some() {
        start();
        render();
        return true;
    }
    false
}
pub fn on_change(t: &Element) {
    match t.id().as_str() {
        "qf" => {
            let v = input_value(t);
            with(|s| s.quiz.filter = v);
            start();
            render();
        }
        "qn" => {
            let v = parse_int(&input_value(t));
            with(|s| s.quiz.n = v as usize);
            start();
            render();
        }
        _ => {}
    }
}
