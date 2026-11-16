//! Interface translation: English UI text is looked up in the ZD dictionary (and ZRX patterns)
//! and shown in Chinese, or as English with Chinese underneath in EN+中文 mode.
use crate::util::*;
use crate::{c, lang, wk_zh, Lang, OBSERVER};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, Node};

const TR_ATTRS: [&str; 4] = ["placeholder", "title", "aria-label", "label"];

thread_local! {
    static MEMO: std::cell::RefCell<std::collections::HashMap<String, Option<String>>> = Default::default();
    static RX: Vec<(js_sys::RegExp, Option<String>)> = c().zrx.iter().map(|(s, f, r)| (js_sys::RegExp::new(s, f), r.clone())).collect();
}

pub fn tr_str(v: &str) -> Option<String> {
    let k = js_trim(v);
    if let Some(r) = MEMO.with(|m| m.borrow().get(k).cloned()) {
        return r;
    }
    let r = tr_str_raw(k);
    MEMO.with(|m| m.borrow_mut().insert(k.to_string(), r.clone()));
    r
}
fn tr_str_raw(k: &str) -> Option<String> {
    if k.is_empty() {
        return None;
    }
    if let Some(z) = c().zd.get(k) {
        return Some(z.clone());
    }
    RX.with(|rx| {
        for (re, rep) in rx {
            re.set_last_index(0);
            if re.test(k) {
                let js = js_sys::JsString::from(k);
                return Some(match rep {
                    Some(r) => String::from(js.replace_by_pattern(re, r)),
                    None => {
                        // (m, a) => wkZh(a) + ' 笔记'
                        let m = re.exec(k).unwrap();
                        let a = m.get(1).as_string().unwrap_or_default();
                        format!("{} 笔记", wk_zh(&a))
                    }
                });
            }
        }
        None
    })
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Translate every text node and attribute below `root` (the original `i18n(root)`).
pub fn translate(root: &Element) {
    let lg = lang();
    if lg == Lang::En {
        return;
    }
    let both = lg == Lang::Both;
    if closest(root, "[data-tr],.bi,.zhb,.zhl,.zhi,textarea,pre.il,.formula").is_some() {
        return;
    }
    let d = doc();
    let walker = match d.create_tree_walker_with_what_to_show(root, 4) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut nodes: Vec<Node> = vec![];
    while let Ok(Some(n)) = walker.next_node() {
        nodes.push(n);
    }
    for t in nodes {
        let p = match t.parent_element() {
            Some(p) => p,
            None => continue,
        };
        if closest(&p, "[data-tr],.bi,.zhb,.zhl,.zhi,script,style,svg,textarea,pre.il,.formula").is_some() {
            continue;
        }
        let nv = t.node_value().unwrap_or_default();
        let z = match tr_str(&nv) {
            Some(z) => z,
            None => continue,
        };
        let en = js_trim(&nv).to_string();
        if p.tag_name() == "OPTION" {
            set_text(&p, &if both { format!("{} / {}", en, z) } else { z });
            continue;
        }
        if both && closest(&p, ".brand").is_some() {
            continue;
        }
        if !both || closest(&p, ".nm,.sn,.sw,.ct,.fbnote").is_some() {
            t.set_node_value(Some(&nv.replacen(&en, &z, 1)));
            continue;
        }
        // trim the part that English and Chinese share at the start / end
        let enu = utf16(&en);
        let mut zs = utf16(&z);
        {
            let mut a = 0;
            while a < enu.len() && a < zs.len() && enu[a] == zs[a] {
                a += 1;
            }
            if a > 0 {
                let pre = String::from_utf16_lossy(&enu[..a]);
                let last = pre.chars().last().unwrap();
                let tail = String::from_utf16_lossy(&zs[a..]);
                if (is_js_space(last) || last == '·') && !js_trim(&tail).is_empty() {
                    zs = zs[a..].to_vec();
                }
            }
            let mut b = 0;
            while b < enu.len() && b < zs.len() && enu[enu.len() - 1 - b] == zs[zs.len() - 1 - b] {
                b += 1;
            }
            if b > 0 {
                let suf = String::from_utf16_lossy(&enu[enu.len() - b..]);
                let first = suf.chars().next().unwrap();
                let head = String::from_utf16_lossy(&zs[..zs.len() - b]);
                if is_js_space(first) && !js_trim(&head).is_empty() {
                    zs = zs[..zs.len() - b].to_vec();
                }
            }
        }
        let zs = String::from_utf16_lossy(&zs);
        let sp = d.create_element("span").unwrap();
        set_attr(&sp, "data-tr", "1");
        sp.set_text_content(Some(&nv));
        let zz = d.create_element("span").unwrap();
        zz.set_class_name("zhi");
        zz.set_text_content(Some(js_trim(&zs)));
        let _ = sp.append_child(&zz);
        let tn: &web_sys::CharacterData = t.unchecked_ref();
        let _ = tn.replace_with_with_node_1(&sp);
    }
    let mut els = vec![root.clone()];
    els.extend(qsa(root, "[placeholder],[title],[aria-label],[label]"));
    for el in els {
        for a in TR_ATTRS {
            let v = match el.get_attribute(a) {
                Some(v) if !v.is_empty() => v,
                _ => continue,
            };
            if let Some(z) = tr_str(&v) {
                set_attr(&el, a, &if both { format!("{} / {}", js_trim(&v), z) } else { z });
            }
        }
    }
}

/// The original translated whatever was added under #app through a MutationObserver.
pub fn handle_records(recs: &js_sys::Array) {
    if lang() == Lang::En {
        return;
    }
    let mut roots: Vec<Element> = vec![];
    for r in recs.iter() {
        let r: web_sys::MutationRecord = r.unchecked_into();
        let added = r.added_nodes();
        for i in 0..added.length() {
            let nd = match added.item(i) {
                Some(n) => n,
                None => continue,
            };
            let el = if nd.node_type() == 1 {
                Some(nd.unchecked_into::<Element>())
            } else if nd.node_type() == 3 {
                nd.parent_element()
            } else {
                None
            };
            if let Some(e) = el {
                if !roots.iter().any(|x| x.is_same_node(Some(&e))) {
                    roots.push(e);
                }
            }
        }
    }
    for r in roots {
        if r.is_connected() {
            translate(&r);
        }
    }
}

pub fn install_observer() {
    let cb = Closure::<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>::new(|recs: js_sys::Array, obs: web_sys::MutationObserver| {
        handle_records(&recs);
        obs.take_records();
    });
    let obs = web_sys::MutationObserver::new(cb.as_ref().unchecked_ref()).unwrap();
    cb.forget();
    let init = web_sys::MutationObserverInit::new();
    init.set_child_list(true);
    init.set_subtree(true);
    let _ = obs.observe_with_options(&app(), &init);
    OBSERVER.with(|o| *o.borrow_mut() = Some(obs));
}

/// trHandle(trObs.takeRecords()) — translate pending additions now.
pub fn flush() {
    let recs = OBSERVER.with(|o| o.borrow().as_ref().map(|o| o.take_records()));
    if let Some(r) = recs {
        handle_records(&r);
    }
}
/// trObs.takeRecords() — drop pending records (our own changes).
pub fn discard() {
    OBSERVER.with(|o| {
        if let Some(o) = o.borrow().as_ref() {
            o.take_records();
        }
    });
}
