//! Content model. Everything the learner reads lives in `content.json`, which is
//! loaded (and, in the desktop app, updated) separately from the program code.
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub type OStr = Option<String>;

#[derive(Deserialize, Clone)]
pub struct Term {
    pub id: String,
    pub name: String,
    pub zh: String,
    pub mods: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct Program {
    pub id: String,
    pub inst: String,
    pub zinst: String,
    pub name: String,
    pub zh: String,
    pub status: String,
    #[serde(default)]
    pub note: OStr,
    #[serde(default)]
    pub znote: OStr,
    pub terms: Vec<Term>,
}

#[derive(Deserialize, Clone)]
pub struct Module {
    pub name: String,
    pub code: String,
    pub zh: String,
}

#[derive(Deserialize, Clone)]
pub struct Section {
    pub h: String,
    pub b: String,
}

#[derive(Deserialize, Clone)]
pub struct Worked {
    pub q: String,
    pub s: String,
}

#[derive(Deserialize, Clone)]
pub struct Topic {
    pub id: String,
    #[serde(rename = "mod")]
    pub module: String,
    pub wk: String,
    pub title: String,
    pub zh: String,
    pub sections: Vec<Section>,
    #[serde(default)]
    pub terms: Option<Vec<Vec<String>>>,
    #[serde(default)]
    pub worked: Option<Vec<Worked>>,
    #[serde(default)]
    pub recap: Option<Vec<Vec<String>>>,
}

/// Chinese text for a topic: `s` sections, `w` worked examples, `r` recap rows.
#[derive(Deserialize, Clone, Default)]
pub struct Zt {
    #[serde(default)]
    pub s: Option<Vec<Option<Vec<OStr>>>>,
    #[serde(default)]
    pub w: Option<Vec<Option<Vec<OStr>>>>,
    #[serde(default)]
    pub r: Option<Vec<Option<Vec<OStr>>>>,
}

pub fn zget(v: &Option<Vec<Option<Vec<OStr>>>>, i: usize, j: usize) -> Option<&str> {
    v.as_ref()?.get(i)?.as_ref()?.get(j)?.as_deref()
}
pub fn zrow(v: &Option<Vec<Option<Vec<OStr>>>>, i: usize) -> Option<&Vec<OStr>> {
    v.as_ref()?.get(i)?.as_ref()
}

#[derive(Deserialize, Clone)]
pub struct Card {
    pub id: String,
    pub topic: String,
    #[serde(default)]
    pub q: OStr,
    #[serde(default)]
    pub a: OStr,
    #[serde(default)]
    pub zq: OStr,
    #[serde(default)]
    pub za: OStr,
    #[serde(default)]
    pub en: OStr,
    #[serde(default)]
    pub zh: OStr,
    #[serde(default)]
    pub term: bool,
}

#[derive(Deserialize, Clone)]
pub struct Anno {
    pub chk: String,
    pub s: Vec<Vec<String>>,
    pub r: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct CalcMeta {
    pub id: String,
    pub title: String,
    pub topic: String,
    #[serde(default)]
    pub seg: Option<Vec<String>>,
    pub fields: Vec<Vec<Value>>,
    #[serde(default)]
    pub text: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct Content {
    pub programs: Vec<Program>,
    pub mods: HashMap<String, Module>,
    pub topics: Vec<Topic>,
    pub zt: HashMap<String, Zt>,
    pub cards: Vec<Card>,
    pub quiz: Vec<Value>,
    pub zq: Vec<Value>,
    pub zd: HashMap<String, String>,
    pub zrx: Vec<(String, String, OStr)>,
    pub fx: Vec<Vec<Value>>,
    pub zf: Vec<Value>,
    pub zg: HashMap<String, String>,
    pub anno: HashMap<String, Anno>,
    pub presets: Map<String, Value>,
    pub zp: HashMap<String, Value>,
    pub chint: HashMap<String, String>,
    pub ohint: HashMap<String, String>,
    pub zchint: HashMap<String, String>,
    pub zohint: HashMap<String, String>,
    pub zkl: HashMap<String, String>,
    pub zil: HashMap<String, String>,
    pub libcat: Vec<(String, String, String, f64)>,
    pub libg: Map<String, Value>,
    pub xsec: HashMap<String, Vec<usize>>,
    pub xwork: HashMap<String, Vec<usize>>,
    pub calcs: Vec<CalcMeta>,
    pub qref: HashMap<String, String>,
    /// Long UI paragraphs kept with the content so they can be updated without a new program build.
    #[serde(default)]
    pub ui: HashMap<String, String>,
    #[serde(default)]
    pub version: OStr,

    // derived
    #[serde(skip)]
    pub by_id: HashMap<String, usize>,
    #[serde(skip)]
    pub mod_order: Vec<String>,
    #[serde(skip)]
    pub mod_term: HashMap<String, (usize, usize)>,
}

impl Content {
    pub fn finish(&mut self) {
        self.by_id = self.topics.iter().enumerate().map(|(i, t)| (t.id.clone(), i)).collect();
        for (pi, p) in self.programs.iter().enumerate() {
            for (ti, t) in p.terms.iter().enumerate() {
                for m in &t.mods {
                    self.mod_order.push(m.clone());
                    self.mod_term.insert(m.clone(), (pi, ti));
                }
            }
        }
    }
    pub fn topic(&self, id: &str) -> Option<&Topic> {
        self.by_id.get(id).map(|&i| &self.topics[i])
    }
    pub fn zt(&self, id: &str) -> Option<&Zt> {
        self.zt.get(id)
    }
    pub fn term_of(&self, m: &str) -> &Term {
        let (p, t) = self.mod_term[m];
        &self.programs[p].terms[t]
    }
}

/// Small accessors for loosely-typed JSON arrays (quiz rows, formula rows).
pub fn vs(v: &Value, i: usize) -> Option<&str> {
    v.get(i).and_then(|x| x.as_str())
}
pub fn va(v: &Value, i: usize) -> Option<&Vec<Value>> {
    v.get(i).and_then(|x| x.as_array())
}
