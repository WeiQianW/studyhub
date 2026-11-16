//! PLC Lab: ladder editor, scan-cycle simulator (contacts, edges, coils, timers, counters) and IL view.
use crate::util::*;
use crate::*;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Element;

fn ser_num<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}
fn ser_onum<S: Serializer>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(x) => ser_num(x, s),
        None => s.serialize_none(),
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Contact {
    pub a: String,
    pub k: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Coil {
    pub t: String,
    pub a: String,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "ser_onum")]
    pub pt: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "ser_onum")]
    pub pv: Option<f64>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Rung {
    #[serde(default)]
    pub c: String,
    pub b: Vec<Vec<Vec<Contact>>>,
    pub o: Coil,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Input {
    pub a: String,
    pub name: String,
    pub mode: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Prog {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    pub inputs: Vec<Input>,
    #[serde(default)]
    pub names: Map<String, Value>,
    pub rungs: Vec<Rung>,
    #[serde(default, serialize_with = "ser_num")]
    pub scan: f64,
    #[serde(default)]
    pub preset: Option<String>,
}

#[derive(Clone, Default)]
struct Tmr {
    et: f64,
    q: bool,
    pt: f64,
    t: String,
}
#[derive(Clone, Default)]
struct Cnt {
    cv: Option<f64>,
    q: bool,
    pv: f64,
    t: String,
}
#[derive(Clone)]
struct CEval {
    pin: bool,
    cl: bool,
}
#[derive(Clone)]
struct BrEval {
    cs: Vec<CEval>,
    out: bool,
}
#[derive(Clone)]
struct BlEval {
    pin: bool,
    out: bool,
    brs: Vec<BrEval>,
}
#[derive(Clone)]
struct REval {
    pw: bool,
    blocks: Vec<BlEval>,
}
#[derive(Clone, Copy, PartialEq)]
pub struct Sel {
    r: usize,
    coil: bool,
    b: usize,
    br: usize,
    c: usize,
}

pub struct LabSt {
    pub p: Prog,
    inp: HashMap<String, bool>,
    img: HashMap<String, bool>,
    prev: HashMap<String, bool>,
    tm: HashMap<String, Tmr>,
    cn: HashMap<String, Cnt>,
    rp: Vec<bool>,
    run: bool,
    scans: u64,
    timer: Option<i32>,
    last: f64,
    sel: Option<Sel>,
    show_il: bool,
    ev: Vec<Option<REval>>,
    confirm: Option<&'static str>,
}

thread_local! {
    static ELS: RefCell<HashMap<String, Element>> = RefCell::new(HashMap::new());
    static TICK: RefCell<Option<Closure<dyn FnMut()>>> = RefCell::new(None);
}

fn preset(k: &str) -> Prog {
    let mut p: Prog = serde_json::from_value(c().presets[k].clone()).unwrap();
    if let Some(z) = c().zp.get(k) {
        if lang() != Lang::En {
            let zc = z.get("c").and_then(|x| x.as_array()).cloned().unwrap_or_default();
            for (i, r) in p.rungs.iter_mut().enumerate() {
                if let Some(zc) = zc.get(i).and_then(|x| x.as_str()) {
                    if !zc.is_empty() && !r.c.is_empty() {
                        r.c = if lang() == Lang::Zh { zc.to_string() } else { format!("{} / {}", r.c, zc) };
                    }
                }
            }
        }
    }
    p
}

impl LabSt {
    pub fn load() -> LabSt {
        let saved = store_get("labProg").and_then(|v| {
            if v.get("rungs").map(|x| x.is_array()).unwrap_or(false) && v.get("inputs").map(|x| x.is_array()).unwrap_or(false) {
                serde_json::from_value::<Prog>(v).ok()
            } else {
                None
            }
        });
        let p = match saved {
            Some(mut s) => {
                if s.scan == 0.0 {
                    s.scan = 50.0;
                }
                s
            }
            None => {
                let mut p = preset("latch");
                p.scan = 50.0;
                p.preset = Some("latch".into());
                p
            }
        };
        LabSt {
            p,
            inp: HashMap::new(),
            img: HashMap::new(),
            prev: HashMap::new(),
            tm: HashMap::new(),
            cn: HashMap::new(),
            rp: vec![],
            run: true,
            scans: 0,
            timer: None,
            last: 0.0,
            sel: None,
            show_il: false,
            ev: vec![],
            confirm: None,
        }
    }
    fn nm(&self, a: &str) -> String {
        if a.starts_with('I') {
            return self.p.inputs.iter().find(|x| x.a == a).map(|x| x.name.clone()).unwrap_or_else(|| "(not wired)".into());
        }
        self.p.names.get(a).and_then(|v| v.as_str()).unwrap_or("").to_string()
    }
    fn bit(&self, a: &str) -> bool {
        if a.starts_with('T') {
            return self.tm.get(a).map(|t| t.q).unwrap_or(false);
        }
        if a.starts_with('C') {
            return self.cn.get(a).map(|t| t.q).unwrap_or(false);
        }
        *self.img.get(a).unwrap_or(&false)
    }
    fn cval(&self, c: &Contact) -> bool {
        let v = self.bit(&c.a);
        let p = *self.prev.get(&c.a).unwrap_or(&false);
        match c.k.as_str() {
            "NO" => v,
            "NC" => !v,
            "P" => v && !p,
            _ => !v && p,
        }
    }
    fn eval_rung(&self, r: &Rung) -> REval {
        let mut pw = true;
        let mut blocks = vec![];
        for bl in &r.b {
            let pin = pw;
            let mut brs = vec![];
            for br in bl {
                let mut p = pin;
                let mut cs = vec![];
                for c in br {
                    let cl = self.cval(c);
                    cs.push(CEval { pin: p, cl });
                    p = p && cl;
                }
                brs.push(BrEval { cs, out: p });
            }
            pw = brs.iter().any(|x| x.out);
            blocks.push(BlEval { pin, out: pw, brs });
        }
        REval { pw, blocks }
    }
    fn tmr(&mut self, a: &str) -> &mut Tmr {
        self.tm.entry(a.to_string()).or_insert_with(|| Tmr { et: 0.0, q: false, pt: 3000.0, t: "TON".into() })
    }
    fn cnt(&mut self, a: &str) -> &mut Cnt {
        self.cn.entry(a.to_string()).or_insert_with(|| Cnt { cv: None, q: false, pv: 3.0, t: "CTU".into() })
    }
    fn scan(&mut self, dt: f64) {
        for i in self.p.inputs.clone() {
            let v = *self.inp.get(&i.a).unwrap_or(&false);
            self.img.insert(i.a.clone(), v);
        }
        self.ev.clear();
        let rungs = self.p.rungs.clone();
        for (ri, r) in rungs.iter().enumerate() {
            let e = self.eval_rung(r);
            let pw = e.pw;
            if self.ev.len() <= ri {
                self.ev.resize(ri + 1, None);
            }
            self.ev[ri] = Some(e);
            if self.rp.len() <= ri {
                self.rp.resize(ri + 1, false);
            }
            let was = self.rp[ri];
            self.rp[ri] = pw;
            let o = &r.o;
            match o.t.as_str() {
                "OUT" => {
                    self.img.insert(o.a.clone(), pw);
                }
                "OUTN" => {
                    self.img.insert(o.a.clone(), !pw);
                }
                "SET" => {
                    if pw {
                        self.img.insert(o.a.clone(), true);
                    }
                }
                "RST" => {
                    if pw {
                        if o.a.starts_with('T') {
                            let t = self.tmr(&o.a);
                            t.et = 0.0;
                            t.q = false;
                        } else if o.a.starts_with('C') {
                            let c = self.cnt(&o.a);
                            c.cv = Some(if c.t == "CTD" { c.pv } else { 0.0 });
                            c.q = if c.t == "CTD" { c.pv <= 0.0 } else { false };
                        } else {
                            self.img.insert(o.a.clone(), false);
                        }
                    }
                }
                "TON" => {
                    let pt = o.pt.unwrap_or(f64::NAN) * 1000.0;
                    let t = self.tmr(&o.a);
                    t.t = "TON".into();
                    t.pt = pt;
                    t.et = if pw { t.pt.min(t.et + dt) } else { 0.0 };
                    t.q = pw && t.et >= t.pt;
                }
                "TOF" => {
                    let pt = o.pt.unwrap_or(f64::NAN) * 1000.0;
                    let t = self.tmr(&o.a);
                    t.t = "TOF".into();
                    t.pt = pt;
                    if pw {
                        t.q = true;
                        t.et = 0.0;
                    } else if t.q {
                        t.et = t.pt.min(t.et + dt);
                        if t.et >= t.pt {
                            t.q = false;
                        }
                    }
                }
                "TONR" => {
                    let pt = o.pt.unwrap_or(f64::NAN) * 1000.0;
                    let t = self.tmr(&o.a);
                    t.t = "TONR".into();
                    t.pt = pt;
                    if pw {
                        t.et = t.pt.min(t.et + dt);
                    }
                    t.q = t.et >= t.pt;
                }
                "CTU" => {
                    let pv = o.pv.unwrap_or(f64::NAN);
                    let c = self.cnt(&o.a);
                    c.t = "CTU".into();
                    c.pv = pv;
                    if c.cv.is_none() {
                        c.cv = Some(0.0);
                    }
                    if pw && !was {
                        c.cv = Some(c.cv.unwrap() + 1.0);
                    }
                    c.q = c.cv.unwrap() >= c.pv;
                }
                "CTD" => {
                    let pv = o.pv.unwrap_or(f64::NAN);
                    let c = self.cnt(&o.a);
                    c.t = "CTD".into();
                    c.pv = pv;
                    if c.cv.is_none() {
                        c.cv = Some(c.pv);
                    }
                    if pw && !was {
                        c.cv = Some((c.cv.unwrap() - 1.0).max(0.0));
                    }
                    c.q = c.cv.unwrap() <= 0.0;
                }
                _ => {}
            }
        }
        let mut snap = self.img.clone();
        for (k, t) in &self.tm {
            snap.insert(k.clone(), t.q);
        }
        for (k, c) in &self.cn {
            snap.insert(k.clone(), c.q);
        }
        self.prev = snap;
        self.scans += 1;
    }
    fn reset_runtime(&mut self) {
        self.inp.clear();
        self.img.clear();
        self.prev.clear();
        self.tm.clear();
        self.cn.clear();
        self.rp.clear();
        self.scans = 0;
        self.ev.clear();
    }
    fn used(&self) -> indexmap::IndexSet<String> {
        let mut u = indexmap::IndexSet::new();
        for r in &self.p.rungs {
            for bl in &r.b {
                for br in bl {
                    for c in br {
                        u.insert(c.a.clone());
                    }
                }
            }
            u.insert(r.o.a.clone());
        }
        u
    }
}

fn range(pre: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{}{}", pre, i)).collect()
}
fn addr(g: &str) -> Vec<String> {
    match g {
        "Q" => range("Q0.", 8),
        "M" => range("M0.", 8),
        "T" => range("T", 8),
        _ => range("C", 8),
    }
}
const SYM: [(&str, &str); 4] = [("NO", "-| |-"), ("NC", "-|/|-"), ("P", "-|P|-"), ("N", "-|N|-")];
const CSYM: [(&str, &str); 9] = [
    ("OUT", "-( )-"),
    ("OUTN", "-(/)-"),
    ("SET", "-(S)-"),
    ("RST", "-(R)-"),
    ("TON", "[TON]"),
    ("TOF", "[TOF]"),
    ("TONR", "[TONR]"),
    ("CTU", "[CTU]"),
    ("CTD", "[CTD]"),
];
fn sym(k: &str) -> &'static str {
    SYM.iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or("undefined")
}
fn csym(k: &str) -> &'static str {
    CSYM.iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or("undefined")
}
fn chint(k: &str) -> (String, Option<String>) {
    (c().chint.get(k).cloned().unwrap_or_else(|| "undefined".into()), c().zchint.get(k).cloned())
}
fn ohint(k: &str) -> (String, Option<String>) {
    (c().ohint.get(k).cloned().unwrap_or_else(|| "undefined".into()), c().zohint.get(k).cloned())
}

fn il_op(o: &str, c: &Contact) -> String {
    format!(
        "{}{} {}{}",
        o,
        if c.k == "NC" { " NOT" } else { "" },
        match c.k.as_str() {
            "P" => "↑",
            "N" => "↓",
            _ => "",
        },
        c.a
    )
}
fn il_branch(br: &[Contact]) -> Vec<String> {
    br.iter().enumerate().map(|(i, c)| il_op(if i > 0 { "AND" } else { "LD" }, c)).collect()
}
fn il_block(bl: &[Vec<Contact>]) -> Vec<String> {
    if bl.len() == 1 {
        return il_branch(&bl[0]);
    }
    if bl.iter().all(|br| br.len() == 1) {
        return bl.iter().enumerate().map(|(i, br)| il_op(if i > 0 { "OR" } else { "LD" }, &br[0])).collect();
    }
    let mut out = vec![];
    for (i, br) in bl.iter().enumerate() {
        out.extend(il_branch(br));
        if i > 0 {
            out.push("OR LD".into());
        }
    }
    out
}
fn il_rung(r: &Rung) -> String {
    let mut l: Vec<String> = vec![];
    if r.b.is_empty() {
        l.push("LD  AlwaysON".into());
    }
    for (i, bl) in r.b.iter().enumerate() {
        if i == 0 {
            l.extend(il_block(bl));
        } else if bl.len() == 1 {
            for c in &bl[0] {
                l.push(il_op("AND", c));
            }
        } else {
            l.extend(il_block(bl));
            l.push("AND LD".into());
        }
    }
    let o = &r.o;
    let n = |x: Option<f64>| x.map(num_str).unwrap_or_else(|| "undefined".into());
    l.push(match o.t.as_str() {
        "OUT" => format!("OUT {}", o.a),
        "OUTN" => format!("OUT NOT {}", o.a),
        "SET" => format!("SET {}", o.a),
        "RST" => format!("RSET {}", o.a),
        t if t.starts_with('T') => format!("{} {}  PT := T#{}s", t, o.a, n(o.pt)),
        t => format!("{} {}  PV := {}", t, o.a, n(o.pv)),
    });
    l.join("\n")
}

fn addr_options(s: &LabSt, sel: &str, groups: &[&str]) -> String {
    let lab = |g: &str| match g {
        "I" => "Inputs",
        "Q" => "Outputs",
        "M" => "Memory bits",
        "T" => "Timer done bits",
        _ => "Counter done bits",
    };
    groups
        .iter()
        .map(|g| {
            let list: Vec<String> = if *g == "I" { s.p.inputs.iter().map(|i| i.a.clone()).collect() } else { addr(g) };
            format!(
                "<optgroup label=\"{}\">{}</optgroup>",
                lab(g),
                list.iter()
                    .map(|a| {
                        let n = s.nm(a);
                        format!(
                            "<option value=\"{}\" {}>{}{}</option>",
                            a,
                            if a == sel { "selected" } else { "" },
                            a,
                            if !n.is_empty() { format!(" · {}", esc(&tlp(&n, zd(&n)))) } else { String::new() }
                        )
                    })
                    .collect::<String>()
            )
        })
        .collect()
}
fn coil_label(o: &Coil) -> String {
    let n = |x: Option<f64>| x.map(num_str).unwrap_or_else(|| "undefined".into());
    if o.t.starts_with('T') {
        format!("{} · {}s", o.a, n(o.pt))
    } else if o.t.starts_with("CT") {
        format!("{} · PV {}", o.a, n(o.pv))
    } else {
        o.a.clone()
    }
}
fn contact_html(s: &LabSt, c: &Contact, id: &str, sel: bool) -> String {
    let (h, z) = chint(&c.k);
    format!(
        "<span class=\"cw\"><i class=\"wire\" data-id=\"{id}-w\"></i><button class=\"ct {sel}\" data-id=\"{id}\" data-pick=\"{id}\" title=\"{t}\"><span class=\"sym\">{sym}</span><span class=\"lb\">{a}</span><span class=\"nm\">{nm}</span></button></span>",
        id = id,
        sel = if sel { "sel" } else { "" },
        t = esc(&tlp(&h, z.as_deref())),
        sym = sym(&c.k),
        a = c.a,
        nm = esc(&s.nm(&c.a))
    )
}
fn rung_html(s: &LabSt, r: &Rung, ri: usize) -> String {
    let sel = s.sel.filter(|x| x.r == ri);
    let sel_c = sel.filter(|x| !x.coil).map(|x| format!("{}-{}-{}-{}", ri, x.b, x.br, x.c));
    let sel_coil = sel.map(|x| x.coil).unwrap_or(false);
    let blocks: String = r
        .b
        .iter()
        .enumerate()
        .map(|(bi, bl)| {
            if bl.len() == 1 {
                bl[0].iter()
                    .enumerate()
                    .map(|(ci, c)| {
                        let id = format!("{}-{}-0-{}", ri, bi, ci);
                        let on = sel_c.as_deref() == Some(id.as_str());
                        contact_html(s, c, &id, on)
                    })
                    .collect::<String>()
            } else {
                format!(
                    "<div class=\"par\" data-id=\"{}-{}-p\">{}</div>",
                    ri,
                    bi,
                    bl.iter()
                        .enumerate()
                        .map(|(bri, br)| format!(
                            "<div class=\"br\">{}<i class=\"wire grow\" data-id=\"{}-{}-{}-o\"></i></div>",
                            br.iter()
                                .enumerate()
                                .map(|(ci, c)| {
                                    let id = format!("{}-{}-{}-{}", ri, bi, bri, ci);
                                    let on = sel_c.as_deref() == Some(id.as_str());
                                    contact_html(s, c, &id, on)
                                })
                                .collect::<String>(),
                            ri,
                            bi,
                            bri
                        ))
                        .collect::<String>()
                )
            }
        })
        .collect();
    let o = &r.o;
    let mut ed = String::new();
    if let (Some(_), Some(x)) = (&sel_c, sel) {
        let c = &r.b[x.b][x.br][x.c];
        let (h, z) = chint(&c.k);
        ed = format!(
            "<div class=\"editor\"><div class=\"edrow\">\n   <div class=\"field\"><label for=\"ed-a\">Address</label><select id=\"ed-a\" data-ed=\"a\">{}</select></div>\n   <div class=\"field\"><label for=\"ed-k\">Contact type</label><select id=\"ed-k\" data-ed=\"k\">{}</select></div></div>\n   <p class=\"hint\">{}</p>\n   <div class=\"edbtns\"><button class=\"chipbtn\" data-ea=\"ser\">＋ Series contact after this</button><button class=\"chipbtn\" data-ea=\"par\">＋ Parallel branch (OR)</button><button class=\"chipbtn\" data-ea=\"after\">＋ Contact after this group</button><button class=\"chipbtn danger\" data-ea=\"del\">Delete contact</button><button class=\"chipbtn\" data-ea=\"close\">Done</button></div></div>",
            addr_options(s, &c.a, &["I", "Q", "M", "T", "C"]),
            SYM.iter()
                .map(|(k, sy)| {
                    let nm = match *k {
                        "NO" => "Normally open",
                        "NC" => "Normally closed",
                        "P" => "Rising edge",
                        _ => "Falling edge",
                    };
                    format!("<option value=\"{}\" {}>{}  {}</option>", k, if *k == c.k { "selected" } else { "" }, sy, tlp(nm, c_zkl(k).as_deref()))
                })
                .collect::<String>(),
            tl(&h, z.as_deref())
        );
    }
    if sel_coil {
        let is_t = o.t.starts_with('T');
        let is_c = o.t.starts_with("CT");
        let groups: Vec<&str> = if is_t {
            vec!["T"]
        } else if is_c {
            vec!["C"]
        } else if o.t == "RST" {
            vec!["Q", "M", "T", "C"]
        } else {
            vec!["Q", "M"]
        };
        let (h, z) = ohint(&o.t);
        let n = |x: Option<f64>| x.map(num_str).unwrap_or_else(|| "undefined".into());
        ed = format!(
            "<div class=\"editor\"><div class=\"edrow\">\n   <div class=\"field\"><label for=\"ed-t\">Instruction</label><select id=\"ed-t\" data-eo=\"t\">{}</select></div>\n   <div class=\"field\"><label for=\"ed-oa\">Address</label><select id=\"ed-oa\" data-eo=\"a\">{}</select></div>\n   {}\n   {}\n   {}</div>\n   <p class=\"hint\">{}</p>\n   <div class=\"edbtns\"><button class=\"chipbtn\" data-ea=\"close\">Done</button></div></div>",
            CSYM.iter()
                .map(|(k, sy)| {
                    let nm = match *k {
                        "OUT" => "Output",
                        "OUTN" => "Negated output",
                        "SET" => "Set (latch)",
                        "RST" => "Reset",
                        "TON" => "On-delay timer",
                        "TOF" => "Off-delay timer",
                        "TONR" => "Accumulative timer",
                        "CTU" => "Count up",
                        _ => "Count down",
                    };
                    format!("<option value=\"{}\" {}>{}  {}</option>", k, if *k == o.t { "selected" } else { "" }, sy, tlp(nm, c().zil.get(*k).map(|s| s.as_str())))
                })
                .collect::<String>(),
            addr_options(s, &o.a, &groups),
            if is_t { format!("<div class=\"field\"><label for=\"ed-pt\">Preset PT (s)</label><input id=\"ed-pt\" data-eo=\"pt\" type=\"number\" min=\"0.1\" step=\"0.1\" value=\"{}\"></div>", n(o.pt)) } else { String::new() },
            if is_c { format!("<div class=\"field\"><label for=\"ed-pv\">Preset PV (count)</label><input id=\"ed-pv\" data-eo=\"pv\" type=\"number\" min=\"1\" step=\"1\" value=\"{}\"></div>", n(o.pv)) } else { String::new() },
            if !is_t && !is_c && !o.a.starts_with('T') && !o.a.starts_with('C') {
                format!(
                    "<div class=\"field\"><label for=\"ed-nm\">Name for {}</label><input id=\"ed-nm\" data-eo=\"nm\" type=\"text\" value=\"{}\" placeholder=\"e.g. Motor\"></div>",
                    o.a,
                    esc(s.p.names.get(&o.a).and_then(|v| v.as_str()).unwrap_or(""))
                )
            } else {
                String::new()
            },
            tl(&h, z.as_deref())
        );
    }
    let (oh, oz) = ohint(&o.t);
    let n_r = s.p.rungs.len();
    format!(
        "<div class=\"card rung\" data-r=\"{ri}\">\n  <div class=\"rhead\"><span class=\"rn\">R{rn}</span><input class=\"cmt\" data-cmt=\"{ri}\" value=\"{cmt}\" placeholder=\"Add a comment for this rung\" aria-label=\"Rung {rn} comment\">\n   <div class=\"rbtns\"><button class=\"chipbtn\" data-ra=\"addc\" title=\"Add a contact at the end\">＋ Contact</button><button class=\"chipbtn\" data-ra=\"up\" title=\"Move up\" {up}>↑</button><button class=\"chipbtn\" data-ra=\"down\" title=\"Move down\" {down}>↓</button><button class=\"chipbtn\" data-ra=\"dup\" title=\"Duplicate\">⧉</button><button class=\"chipbtn danger\" data-ra=\"del\" title=\"Delete rung\">✕</button></div></div>\n  <div class=\"ladder\"><div class=\"lad p\">{blocks}<i class=\"wire grow\" data-id=\"{ri}-end\"></i><button class=\"ct coil {cs}\" data-id=\"{ri}-coil\" data-coil=\"{ri}\" title=\"{ot}\"><span class=\"sym\">{csym}</span><span class=\"lb\">{cl}</span><span class=\"nm\">{onm}</span></button><i class=\"wire\" data-id=\"{ri}-end2\"></i></div></div>\n  {ed}\n  {il}\n </div>",
        ri = ri,
        rn = ri + 1,
        cmt = esc(&r.c),
        up = if ri > 0 { "" } else { "disabled" },
        down = if ri + 1 < n_r { "" } else { "disabled" },
        blocks = if blocks.is_empty() { "<span class=\"muted\" style=\"font-size:12px;padding:0 8px\">always ON</span>".to_string() } else { blocks },
        cs = if sel_coil { "sel" } else { "" },
        ot = esc(&tlp(&oh, oz.as_deref())),
        csym = csym(&o.t),
        cl = coil_label(o),
        onm = esc(&s.nm(&o.a)),
        ed = ed,
        il = if s.show_il { format!("<pre class=\"il\">{}</pre>", esc(&il_rung(r))) } else { String::new() }
    )
}
fn c_zkl(k: &str) -> Option<String> {
    c().zkl.get(k).cloned()
}

fn prog_json() -> String {
    with(|s| serde_json::to_string(&s.lab.p).unwrap())
}
fn save_prog() {
    let j = prog_json();
    ls_set("dpsh_labProg", &j);
}

pub fn render() {
    let presets = &c().presets;
    let (cur_preset, run, scan, show_il) = with(|s| (s.lab.p.preset.clone(), s.lab.run, s.lab.p.scan, s.lab.show_il));
    let preset_opts: String = presets
        .iter()
        .map(|(k, v)| {
            let zt = c().zp.get(k).and_then(|z| z.get("title")).and_then(|x| x.as_str());
            format!("<option value=\"{}\" {}>{}</option>", k, if cur_preset.as_deref() == Some(k.as_str()) { "selected" } else { "" }, tlp(v["title"].as_str().unwrap_or(""), zt))
        })
        .collect();
    let desc = match cur_preset.as_ref().and_then(|k| presets.get(k).map(|v| (k, v))) {
        Some((k, v)) => format!(
            "<p class=\"muted\" id=\"lDesc\" style=\"margin:10px 2px 0;max-width:90ch\">{}</p>",
            tl(v["desc"].as_str().unwrap_or(""), c().zp.get(k).and_then(|z| z.get("desc")).and_then(|x| x.as_str()))
        ),
        None => String::new(),
    };
    let h = format!(
        "<div class=\"toolbar\"><h2 style=\"margin-right:auto\">PLC Lab</h2><span class=\"muted\" style=\"font-size:13px\">Build, edit and run your own ladder program. Click any contact or coil to edit it.</span></div>\n <div class=\"card labbar\">\n  <div class=\"field\" style=\"min-width:220px;flex:1\"><label for=\"lp\">Example program</label><select id=\"lp\">{po}</select></div>\n  <button class=\"btn ghost\" id=\"lpLoad\">Load example</button>\n  <span class=\"sep\"></span>\n  <button class=\"btn\" id=\"lRun\">{rl}</button>\n  <button class=\"btn ghost\" id=\"lStep\" {sd}>Single scan</button>\n  <div class=\"field\"><label for=\"lScan\">Scan time</label><select id=\"lScan\">{so}</select></div>\n  <button class=\"btn ghost\" id=\"lReset\" title=\"Clear all bits, timers and counters\">Reset runtime</button>\n  <button class=\"chipbtn\" id=\"lIL\" aria-pressed=\"{il}\">Show instruction list</button>\n  <span class=\"muted mono\" id=\"lScans\"></span>\n </div>\n {desc}\n <div class=\"lab2\">\n  <div class=\"card panel\" id=\"lPanel\"></div>\n  <div style=\"min-width:0\"><div class=\"rungs\" id=\"rungs\"></div>\n   <div style=\"display:flex;gap:10px;flex-wrap:wrap;margin-top:14px\"><button class=\"btn ghost\" id=\"addRung\">＋ Add rung</button><button class=\"btn ghost\" id=\"clearProg\">Clear program</button></div>\n   <details class=\"card cfg\" id=\"cfgIO\"><summary>Configure inputs &amp; names</summary><div id=\"cfgBody\"></div></details>\n   <details class=\"card cfg\"><summary>Save or share program as text</summary>\n    <p class=\"muted\" style=\"font-size:13px\">Your program saves automatically in this browser. Copy this text to keep a backup or send it to a classmate; paste text here and press Import to load one.</p>\n    <textarea id=\"ioText\" rows=\"6\" spellcheck=\"false\"></textarea>\n    <div style=\"display:flex;gap:8px;flex-wrap:wrap;margin-top:8px\"><button class=\"chipbtn\" id=\"ioCopy\">Copy</button><button class=\"chipbtn\" id=\"ioImport\">Import</button><span class=\"muted\" id=\"ioMsg\" style=\"font-size:13px\"></span></div></details>\n  </div>\n </div>",
        po = preset_opts,
        rl = if run { "❚❚ Pause" } else { "▶ Run" },
        sd = if run { "disabled" } else { "" },
        so = [10.0, 50.0, 100.0, 250.0, 500.0, 1000.0].iter().map(|&v| format!("<option value=\"{}\" {}>{} ms</option>", num_str(v), if scan == v { "selected" } else { "" }, num_str(v))).collect::<String>(),
        il = show_il,
        desc = desc
    );
    set_html(&app(), &h);
    render_panel();
    render_rungs();
    render_cfg();
    if let Some(t) = by_id("ioText") {
        set_input_value(&t, &prog_json());
    }
    start_loop();
}

fn changed() {
    with(|s| {
        if let Some(p) = &s.lab.p.preset {
            if !c().presets.contains_key(p) {
                s.lab.p.preset = None;
            }
        }
    });
    save_prog();
    render_rungs();
    render_panel();
    if let Some(t) = by_id("ioText") {
        set_input_value(&t, &prog_json());
    }
}

fn render_rungs() {
    let el = match by_id("rungs") {
        Some(e) => e,
        None => return,
    };
    let h = with(|s| {
        if s.lab.p.rungs.is_empty() {
            "<div class=\"card\" style=\"padding:24px;text-align:center\"><p class=\"muted\">No rungs yet. Add a rung to start your program.</p></div>".to_string()
        } else {
            s.lab.p.rungs.iter().enumerate().map(|(i, r)| rung_html(&s.lab, r, i)).collect()
        }
    });
    set_html(&el, &h);
    let mut m = HashMap::new();
    for n in qsa(&el, "[data-id]") {
        m.insert(attr(&n, "data-id"), n);
    }
    ELS.with(|e| *e.borrow_mut() = m);
    live();
}

fn render_panel() {
    let el = match by_id("lPanel") {
        Some(e) => e,
        None => return,
    };
    let h = with(|s| {
        let sw: String = s
            .lab
            .p
            .inputs
            .iter()
            .map(|i| {
                format!(
                    "<button class=\"sw\" data-in=\"{}\"><span class=\"led\"></span><span style=\"min-width:0\"><span class=\"ad\">{}</span><small><span>{}</span> · <span>{}</span></small></span></button>",
                    i.a,
                    i.a,
                    esc(&i.name),
                    if i.mode == "push" { "push button" } else { "switch" }
                )
            })
            .collect();
        format!(
            "<h4>Inputs</h4><div class=\"swl\">{}</div>\n <h4>Outputs &amp; status</h4><div id=\"lStat\"></div>",
            if sw.is_empty() { "<p class=\"muted\">No inputs. Add one under Configure.</p>".into() } else { sw }
        )
    });
    set_html(&el, &h);
}

fn stat_html(s: &LabSt) -> String {
    let u = s.used();
    let mut rows = vec![];
    let mut qm = addr("Q");
    qm.extend(addr("M"));
    for a in qm.iter().filter(|a| u.contains(*a) || s.p.names.get(*a).and_then(|v| v.as_str()).map(|x| !x.is_empty()).unwrap_or(false)) {
        rows.push(format!(
            "<div class=\"srow\"><span class=\"lamp {}\"></span><span class=\"mono\">{}</span><span class=\"sn\">{}</span></div>",
            if *s.img.get(a).unwrap_or(&false) { "on" } else { "" },
            a,
            esc(&s.nm(a))
        ));
    }
    for a in addr("T").iter().filter(|a| u.contains(*a)) {
        let t = s.tm.get(a).cloned().unwrap_or(Tmr { et: 0.0, pt: 3000.0, q: false, t: "TON".into() });
        rows.push(format!(
            "<div class=\"srow\"><span class=\"lamp {}\"></span><span class=\"mono\">{}</span><span class=\"sn\">{} {} / {} s</span></div><div class=\"tbar\"><i style=\"width:{}%\"></i></div>",
            if t.q { "on" } else { "" },
            a,
            t.t,
            to_fixed(t.et / 1000.0, 1),
            to_fixed(t.pt / 1000.0, 1),
            num_str(if t.pt != 0.0 && !t.pt.is_nan() { t.et / t.pt * 100.0 } else { 0.0 })
        ));
    }
    for a in addr("C").iter().filter(|a| u.contains(*a)) {
        let c = s.cn.get(a).cloned().unwrap_or(Cnt { cv: Some(0.0), pv: 0.0, q: false, t: "CTU".into() });
        rows.push(format!(
            "<div class=\"srow\"><span class=\"lamp {}\"></span><span class=\"mono\">{}</span><span class=\"sn\">{} CV {} / PV {}</span></div>",
            if c.q { "on" } else { "" },
            a,
            c.t,
            num_str(c.cv.unwrap_or(0.0)),
            num_str(c.pv)
        ));
    }
    if rows.is_empty() {
        "<p class=\"muted\">Nothing used yet.</p>".into()
    } else {
        rows.join("")
    }
}

fn live() {
    if cur_tab() != "lab" {
        return;
    }
    let (stat, scans, inp, ev, shape) = with(|s| {
        let shape: Vec<Vec<usize>> = s.lab.p.rungs.iter().map(|r| r.b.iter().map(|bl| bl.len()).collect()).collect();
        (stat_html(&s.lab), s.lab.scans, s.lab.inp.clone(), s.lab.ev.clone(), shape)
    });
    for b in dqsa("#lPanel .sw") {
        toggle_class(&b, "on", *inp.get(&attr(&b, "data-in")).unwrap_or(&false));
    }
    if let Some(st) = by_id("lStat") {
        set_html(&st, &stat);
    }
    if let Some(sc) = by_id("lScans") {
        set_text(&sc, &format!("scan #{}", scans));
    }
    ELS.with(|els| {
        let els = els.borrow();
        let set = |id: String, cls: &str, v: bool| {
            if let Some(n) = els.get(&id) {
                toggle_class(n, cls, v);
            }
        };
        for (ri, e) in ev.iter().enumerate() {
            let e = match e {
                Some(e) => e,
                None => continue,
            };
            let rs = match shape.get(ri) {
                Some(r) => r,
                None => continue,
            };
            for (bi, bl) in e.blocks.iter().enumerate() {
                let multi = rs.get(bi).map(|n| *n > 1).unwrap_or(false);
                if multi {
                    set(format!("{}-{}-p", ri, bi), "e", bl.pin);
                    set(format!("{}-{}-p", ri, bi), "eo", bl.out);
                }
                for (bri, br) in bl.brs.iter().enumerate() {
                    if multi {
                        set(format!("{}-{}-{}-o", ri, bi, bri), "e", br.out);
                    }
                    for (ci, c) in br.cs.iter().enumerate() {
                        set(format!("{}-{}-{}-{}-w", ri, bi, bri, ci), "e", c.pin);
                        set(format!("{}-{}-{}-{}", ri, bi, bri, ci), "cl", c.cl);
                    }
                }
            }
            set(format!("{}-end", ri), "e", e.pw);
            set(format!("{}-end2", ri), "e", e.pw);
            set(format!("{}-coil", ri), "on", e.pw);
        }
    });
}

fn start_loop() {
    stop();
    let (empty, run, scan) = with(|s| {
        s.lab.last = now();
        (s.lab.ev.is_empty(), s.lab.run, s.lab.p.scan)
    });
    if empty {
        with(|s| s.lab.scan(0.0));
    }
    live();
    if run {
        TICK.with(|t| {
            if t.borrow().is_none() {
                *t.borrow_mut() = Some(Closure::<dyn FnMut()>::new(|| {
                    let n = now();
                    with(|s| {
                        let dt = n - s.lab.last;
                        s.lab.last = n;
                        s.lab.scan(dt);
                    });
                    live();
                }));
            }
            let id = win()
                .set_interval_with_callback_and_timeout_and_arguments_0(t.borrow().as_ref().unwrap().as_ref().unchecked_ref(), scan as i32)
                .unwrap_or(0);
            with(|s| s.lab.timer = Some(id));
        });
    }
}
pub fn stop() {
    let t = with(|s| s.lab.timer.take());
    if let Some(id) = t {
        win().clear_interval_with_handle(id);
    }
}

fn render_cfg() {
    let el = match by_id("cfgBody") {
        Some(e) => e,
        None => return,
    };
    let h = with(|s| {
        let p = &s.lab.p;
        let rows: String = p
            .inputs
            .iter()
            .enumerate()
            .map(|(n, i)| {
                format!(
                    "<div class=\"cfgrow\"><span class=\"mono\">{a}</span>\n   <input type=\"text\" data-in-name=\"{n}\" value=\"{nm}\" aria-label=\"Name for {a}\">\n   <select data-in-mode=\"{n}\" aria-label=\"Type for {a}\"><option value=\"toggle\" {t}>Switch (stays on)</option><option value=\"push\" {pu}>Push button (on while held)</option></select>\n   <button class=\"chipbtn danger\" data-in-del=\"{n}\" aria-label=\"Remove {a}\">✕</button></div>",
                    a = i.a,
                    n = n,
                    nm = esc(&i.name),
                    t = if i.mode == "toggle" { "selected" } else { "" },
                    pu = if i.mode == "push" { "selected" } else { "" }
                )
            })
            .collect();
        let mut qm = addr("Q");
        qm.extend(addr("M"));
        let names: String = qm
            .iter()
            .map(|a| {
                format!(
                    "<div class=\"cfgrow\"><span class=\"mono\">{a}</span><input type=\"text\" data-nm=\"{a}\" value=\"{v}\" placeholder=\"unnamed\" aria-label=\"Name for {a}\"></div>",
                    a = a,
                    v = esc(p.names.get(a).and_then(|v| v.as_str()).unwrap_or(""))
                )
            })
            .collect();
        format!(
            "<h4>Inputs</h4><div class=\"cfgrows\">{}</div>\n  {}\n  <h4>Output &amp; memory names</h4><div class=\"cfgrows two\">{}</div>\n  <p class=\"muted\" style=\"font-size:13px\">Push buttons are on only while you hold them (mouse, finger, or Space/Enter). Use NC contacts in the program for stop buttons, as in the real starter circuits.</p>",
            rows,
            if p.inputs.len() < 8 { "<button class=\"chipbtn\" id=\"inAdd\" style=\"margin-top:8px\">＋ Add input</button>" } else { "" },
            names
        )
    });
    set_html(&el, &h);
}

fn confirm_btn(id: &'static str, which: &'static str, ask: &str, normal: &'static str) -> bool {
    let b = match by_id(id) {
        Some(b) => b,
        None => return false,
    };
    if with(|s| s.lab.confirm) != Some(which) {
        with(|s| s.lab.confirm = Some(which));
        set_text(&b, ask);
        add_class(&b, "warn");
        timeout(3000, move || {
            if with(|s| s.lab.confirm) == Some(which) {
                with(|s| s.lab.confirm = None);
                set_text(&b, normal);
                rm_class(&b, "warn");
            }
        });
        return false;
    }
    with(|s| s.lab.confirm = None);
    true
}

fn first_input() -> String {
    with(|s| s.lab.p.inputs.first().map(|i| i.a.clone()).unwrap_or_else(|| "I0.0".into()))
}
fn ct(a: &str) -> Contact {
    Contact { a: a.into(), k: "NO".into() }
}

pub fn on_click(t: &Element) -> bool {
    let id_is = |id: &str| closest(t, &format!("#{}", id)).is_some();
    if id_is("lpLoad") {
        if !confirm_btn("lpLoad", "load", "Click again to replace your program", "Load example") {
            return true;
        }
        let k = by_id("lp").map(|e| input_value(&e)).unwrap_or_default();
        if !c().presets.contains_key(&k) {
            return true;
        }
        let mut p = preset(&k);
        p.scan = 50.0;
        p.preset = Some(k);
        with(|s| {
            s.lab.p = p;
            s.lab.sel = None;
            s.lab.reset_runtime();
        });
        save_prog();
        render();
        return true;
    }
    if id_is("lRun") {
        with(|s| s.lab.run = !s.lab.run);
        render();
        return true;
    }
    if id_is("lStep") {
        with(|s| {
            let sc = s.lab.p.scan;
            s.lab.scan(sc)
        });
        live();
        return true;
    }
    if id_is("lReset") {
        with(|s| {
            s.lab.reset_runtime();
            s.lab.scan(0.0);
        });
        live();
        return true;
    }
    if id_is("lIL") {
        let v = with(|s| {
            s.lab.show_il = !s.lab.show_il;
            s.lab.show_il
        });
        if let Some(b) = by_id("lIL") {
            set_attr(&b, "aria-pressed", if v { "true" } else { "false" });
        }
        render_rungs();
        return true;
    }
    if id_is("addRung") {
        let a = first_input();
        with(|s| {
            s.lab.p.rungs.push(Rung { c: String::new(), b: vec![vec![vec![ct(&a)]]], o: Coil { t: "OUT".into(), a: "Q0.0".into(), pt: None, pv: None } });
            let r = s.lab.p.rungs.len() - 1;
            s.lab.sel = Some(Sel { r, coil: true, b: 0, br: 0, c: 0 });
        });
        changed();
        return true;
    }
    if id_is("clearProg") {
        if !confirm_btn("clearProg", "clear", "Click again to delete all rungs", "Clear program") {
            return true;
        }
        with(|s| {
            s.lab.p.rungs.clear();
            s.lab.p.preset = None;
            s.lab.sel = None;
            s.lab.reset_runtime();
        });
        changed();
        if let Some(cp) = by_id("clearProg") {
            set_text(&cp, "Clear program");
            rm_class(&cp, "warn");
        }
        return true;
    }
    if id_is("ioCopy") {
        let tx = by_id("ioText").unwrap();
        let j = prog_json();
        set_input_value(&tx, &j);
        let m = by_id("ioMsg").unwrap();
        let p = win().navigator().clipboard().write_text(&j);
        let m2 = m.clone();
        let tx2 = tx.clone();
        let ok = Closure::once(move |_v: JsValue| set_text(&m, "Copied."));
        let bad = Closure::once(move |_v: JsValue| {
            if let Some(t) = tx2.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                t.select();
            }
            set_text(&m2, "Text selected — press Ctrl/Cmd + C.");
        });
        let _ = p.then2(&ok, &bad);
        ok.forget();
        bad.forget();
        return true;
    }
    if id_is("ioImport") {
        let m = by_id("ioMsg").unwrap();
        let txt = by_id("ioText").map(|e| input_value(&e)).unwrap_or_default();
        let parsed = serde_json::from_str::<Value>(&txt).ok().filter(|v| v.get("rungs").map(|x| x.is_array()).unwrap_or(false) && v.get("inputs").map(|x| x.is_array()).unwrap_or(false)).and_then(|v| serde_json::from_value::<Prog>(v).ok());
        match parsed {
            Some(mut p) => {
                if p.scan == 0.0 {
                    p.scan = 50.0;
                }
                p.preset = None;
                with(|s| {
                    s.lab.p = p;
                    s.lab.sel = None;
                    s.lab.reset_runtime();
                });
                save_prog();
                render();
            }
            None => set_text(&m, "That text is not a valid program. Copy the full text from the box above."),
        }
        return true;
    }
    // rung editor
    if closest(t, "#rungs").is_some() {
        if let Some(pk) = closest(t, "[data-pick]") {
            let v: Vec<usize> = attr(&pk, "data-pick").split('-').map(|x| x.parse().unwrap_or(0)).collect();
            with(|s| s.lab.sel = Some(Sel { r: v[0], coil: false, b: v[1], br: v[2], c: v[3] }));
            render_rungs();
            return true;
        }
        if let Some(cl) = closest(t, "[data-coil]") {
            let r: usize = attr(&cl, "data-coil").parse().unwrap_or(0);
            with(|s| s.lab.sel = Some(Sel { r, coil: true, b: 0, br: 0, c: 0 }));
            render_rungs();
            return true;
        }
        if let Some(ra) = closest(t, "[data-ra]") {
            if ra.has_attribute("disabled") {
                return true;
            }
            let ri: usize = closest(&ra, "[data-r]").map(|x| attr(&x, "data-r").parse().unwrap_or(0)).unwrap_or(0);
            let act = attr(&ra, "data-ra");
            let a0 = first_input();
            if act == "addc" {
                with(|s| {
                    let r = &mut s.lab.p.rungs[ri];
                    r.b.push(vec![vec![ct(&a0)]]);
                    let b = r.b.len() - 1;
                    s.lab.sel = Some(Sel { r: ri, coil: false, b, br: 0, c: 0 });
                });
                changed();
                return true;
            }
            with(|s| {
                let rs = &mut s.lab.p.rungs;
                match act.as_str() {
                    "up" if ri > 0 => rs.swap(ri - 1, ri),
                    "down" if ri + 1 < rs.len() => rs.swap(ri + 1, ri),
                    "dup" => {
                        let x = rs[ri].clone();
                        rs.insert(ri + 1, x);
                    }
                    "del" => {
                        rs.remove(ri);
                    }
                    _ => {}
                }
                s.lab.sel = None;
                s.lab.rp.clear();
            });
            changed();
            return true;
        }
        if let Some(ea) = closest(t, "[data-ea]") {
            let sel = with(|s| s.lab.sel);
            let x = match sel {
                Some(x) => x,
                None => return true,
            };
            let act = attr(&ea, "data-ea");
            if act == "close" {
                with(|s| s.lab.sel = None);
                render_rungs();
                return true;
            }
            with(|s| {
                let r = &mut s.lab.p.rungs[x.r];
                let a = r.b[x.b][x.br][x.c].a.clone();
                match act.as_str() {
                    "ser" => {
                        r.b[x.b][x.br].insert(x.c + 1, ct(&a));
                        s.lab.sel = Some(Sel { c: x.c + 1, ..x });
                    }
                    "par" => {
                        r.b[x.b].insert(x.br + 1, vec![ct(&a)]);
                        s.lab.sel = Some(Sel { br: x.br + 1, c: 0, ..x });
                    }
                    "after" => {
                        r.b.insert(x.b + 1, vec![vec![ct(&a)]]);
                        s.lab.sel = Some(Sel { r: x.r, coil: false, b: x.b + 1, br: 0, c: 0 });
                    }
                    "del" => {
                        r.b[x.b][x.br].remove(x.c);
                        if r.b[x.b][x.br].is_empty() {
                            r.b[x.b].remove(x.br);
                        }
                        if r.b[x.b].is_empty() {
                            r.b.remove(x.b);
                        }
                        s.lab.sel = None;
                    }
                    _ => {}
                }
            });
            changed();
            return true;
        }
        return false;
    }
    // inputs panel (switch mode toggles on click)
    if let Some(b) = closest(t, "#lPanel .sw") {
        let a = attr(&b, "data-in");
        let toggle = with(|s| s.lab.p.inputs.iter().find(|i| i.a == a).map(|i| i.mode != "push").unwrap_or(false));
        if toggle {
            with(|s| {
                let v = !*s.lab.inp.get(&a).unwrap_or(&false);
                s.lab.inp.insert(a, v);
            });
            live();
        }
        return true;
    }
    if let Some(b) = closest(t, "#cfgBody [data-in-del]") {
        let n: usize = attr(&b, "data-in-del").parse().unwrap_or(0);
        with(|s| {
            if n < s.lab.p.inputs.len() {
                let i = s.lab.p.inputs.remove(n);
                s.lab.inp.remove(&i.a);
            }
        });
        changed();
        render_panel();
        render_cfg();
        return true;
    }
    if id_is("inAdd") {
        with(|s| {
            let all = range("I0.", 8);
            if let Some(a) = all.iter().find(|x| !s.lab.p.inputs.iter().any(|i| &i.a == *x)) {
                s.lab.p.inputs.push(Input { a: a.clone(), name: format!("Input {}", &a[3..]), mode: "toggle".into() });
                s.lab.p.inputs.sort_by(|x, y| x.a.cmp(&y.a));
            }
        });
        changed();
        render_panel();
        render_cfg();
        return true;
    }
    false
}

pub fn on_change(t: &Element) {
    if t.id() == "lScan" {
        let v = js_number(&input_value(t));
        with(|s| s.lab.p.scan = v);
        save_prog();
        start_loop();
        return;
    }
    if closest(t, "#rungs").is_some() {
        if let Some(r) = dataset(t, "cmt") {
            let ri: usize = r.parse().unwrap_or(0);
            let v = input_value(t);
            with(|s| {
                if let Some(r) = s.lab.p.rungs.get_mut(ri) {
                    r.c = v;
                }
            });
            save_prog();
            return;
        }
        let sel = with(|s| s.lab.sel);
        if let (Some(k), Some(x)) = (dataset(t, "ed"), sel) {
            let v = input_value(t);
            with(|s| {
                let c = &mut s.lab.p.rungs[x.r].b[x.b][x.br][x.c];
                if k == "a" {
                    c.a = v;
                } else {
                    c.k = v;
                }
            });
            changed();
            return;
        }
        if let (Some(k), Some(x)) = (dataset(t, "eo"), sel) {
            let v = input_value(t);
            with(|s| {
                let used = s.lab.used();
                let names = &mut s.lab.p.names;
                let o = &mut s.lab.p.rungs[x.r].o;
                match k.as_str() {
                    "t" => {
                        o.t = v.clone();
                        let is_t = o.t.starts_with('T');
                        let is_c = o.t.starts_with("CT");
                        if is_t {
                            if !o.a.starts_with('T') {
                                o.a = addr("T").into_iter().find(|a| !used.contains(a)).unwrap_or_else(|| "T0".into());
                            }
                            if !o.pt.map(|p| p != 0.0 && !p.is_nan()).unwrap_or(false) {
                                o.pt = Some(3.0);
                            }
                        } else if is_c {
                            if !o.a.starts_with('C') {
                                o.a = addr("C").into_iter().find(|a| !used.contains(a)).unwrap_or_else(|| "C0".into());
                            }
                            if !o.pv.map(|p| p != 0.0 && !p.is_nan()).unwrap_or(false) {
                                o.pv = Some(3.0);
                            }
                        } else if o.t != "RST" && !(o.a.starts_with('Q') || o.a.starts_with('M')) {
                            o.a = "Q0.0".into();
                        }
                    }
                    "a" => o.a = v,
                    "pt" => {
                        let f = parse_float(&v);
                        o.pt = Some(0.1f64.max(if f.is_nan() || f == 0.0 { 1.0 } else { f }));
                    }
                    "pv" => {
                        let f = parse_int(&v);
                        o.pv = Some(1f64.max(if f.is_nan() || f == 0.0 { 1.0 } else { f }));
                    }
                    "nm" => {
                        let tv = js_trim(&v).to_string();
                        if !tv.is_empty() {
                            names.insert(o.a.clone(), Value::String(tv));
                        } else {
                            names.shift_remove(&o.a);
                        }
                    }
                    _ => {}
                }
            });
            changed();
        }
        return;
    }
    if let Some(n) = dataset(t, "in-name") {
        let n: usize = n.parse().unwrap_or(0);
        let v = input_value(t);
        with(|s| {
            if let Some(i) = s.lab.p.inputs.get_mut(n) {
                i.name = if v.is_empty() { i.a.clone() } else { v };
            }
        });
        changed();
        render_panel();
        return;
    }
    if let Some(n) = dataset(t, "in-mode") {
        let n: usize = n.parse().unwrap_or(0);
        let v = input_value(t);
        with(|s| {
            if let Some(i) = s.lab.p.inputs.get_mut(n) {
                i.mode = v;
                let a = i.a.clone();
                s.lab.inp.insert(a, false);
            }
        });
        save_prog();
        render_panel();
        live();
        return;
    }
    if let Some(a) = dataset(t, "nm") {
        let v = js_trim(&input_value(t)).to_string();
        with(|s| {
            if !v.is_empty() {
                s.lab.p.names.insert(a, Value::String(v));
            } else {
                s.lab.p.names.shift_remove(&a);
            }
        });
        changed();
    }
}

fn push_input(t: &Element) -> Option<String> {
    let b = closest(t, "#lPanel .sw")?;
    let a = attr(&b, "data-in");
    let push = with(|s| s.lab.p.inputs.iter().any(|i| i.a == a && i.mode == "push"));
    if push {
        Some(a)
    } else {
        None
    }
}
fn press(a: String, on: bool) {
    let changed = with(|s| {
        let cur = *s.lab.inp.get(&a).unwrap_or(&false);
        if on {
            s.lab.inp.insert(a, true);
            true
        } else if cur {
            s.lab.inp.insert(a, false);
            true
        } else {
            false
        }
    });
    if changed {
        live();
    }
}
pub fn on_pointer(e: &web_sys::Event, t: &Element, kind: &str) {
    // pointerleave does not bubble: only react when it is the button itself
    if kind == "pointerleave" && !t.matches("#lPanel .sw").unwrap_or(false) {
        return;
    }
    if let Some(a) = push_input(t) {
        if kind == "pointerdown" {
            e.prevent_default();
            press(a, true);
        } else {
            press(a, false);
        }
    }
}
pub fn on_key(e: &web_sys::KeyboardEvent, t: &Element, down: bool) {
    let k = e.key();
    if k != " " && k != "Enter" {
        return;
    }
    if let Some(a) = push_input(t) {
        if down {
            e.prevent_default();
            press(a, true);
        } else {
            press(a, false);
        }
    }
}
