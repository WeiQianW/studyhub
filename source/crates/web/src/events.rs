//! One delegated listener per event type, routed to the active tab.
use crate::util::*;
use crate::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, Event};

fn target(e: &Event) -> Option<Element> {
    let t = e.target()?;
    if let Ok(el) = t.clone().dyn_into::<Element>() {
        return Some(el);
    }
    // text nodes etc.
    t.dyn_into::<web_sys::Node>().ok()?.parent_element()
}

fn listen(on: &web_sys::EventTarget, ev: &str, capture: bool, f: impl FnMut(Event) + 'static) {
    let cb = Closure::<dyn FnMut(Event)>::new(f);
    let _ = on.add_event_listener_with_callback_and_bool(ev, cb.as_ref().unchecked_ref(), capture);
    cb.forget();
}

pub fn install() {
    let d: web_sys::EventTarget = doc().into();

    listen(&d, "click", false, |e| {
        let t = match target(&e) {
            Some(t) => t,
            None => return,
        };
        let me: &web_sys::MouseEvent = e.unchecked_ref();
        if search::on_click(&t, me) {
            return;
        }
        if let Some(b) = closest(&t, "#tabs button") {
            go(&attr(&b, "data-tab"), GoOpts::default());
            return;
        }
        if let Some(b) = closest(&t, "#langSeg [data-lang]") {
            set_language(&attr(&b, "data-lang"));
            return;
        }
        if closest(&t, "#themeBtn").is_some() {
            toggle_theme();
            return;
        }
        if closest(&t, "#toTop").is_some() {
            nav::to_top();
            return;
        }
        if !app().contains(Some(&t)) {
            return;
        }
        let handled = match cur_tab().as_str() {
            "home" => home::on_click(&t),
            "learn" => learn::on_click(&t),
            "cards" => cards::on_click(&t),
            "quiz" => quiz::on_click(&t),
            "calc" => calc::on_click(&t),
            "lab" => lab::on_click(&t),
            "formulas" => formulas::on_click(&t),
            "extra" => extra::on_click(&t),
            _ => false,
        };
        let _ = handled;
        // app-wide listeners of the original (they ran after the element's own handler)
        if let Some(b) = closest(&t, "[data-sec-go]") {
            let i: usize = attr(&b, "data-sec-go").parse().unwrap_or(0);
            nav::sec_go(i);
        }
        notebook::on_mark_click(&t);
    });

    listen(&d, "input", false, |e| {
        let t = match target(&e) {
            Some(t) => t,
            None => return,
        };
        if search::on_input(&t) {
            return;
        }
        if !app().contains(Some(&t)) {
            return;
        }
        match cur_tab().as_str() {
            "learn" => learn::on_input(&t),
            "calc" => calc::on_input(&t),
            "formulas" => formulas::on_input(&t),
            "extra" => extra::on_input(&t),
            _ => {}
        }
    });

    listen(&d, "change", false, |e| {
        let t = match target(&e) {
            Some(t) => t,
            None => return,
        };
        if !app().contains(Some(&t)) {
            return;
        }
        match cur_tab().as_str() {
            "learn" => learn::on_change(&t),
            "cards" => cards::on_change(&t),
            "quiz" => quiz::on_change(&t),
            "lab" => lab::on_change(&t),
            _ => {}
        }
    });

    listen(&d, "keydown", false, |e| {
        let t = match target(&e) {
            Some(t) => t,
            None => return,
        };
        let ke: &web_sys::KeyboardEvent = e.unchecked_ref();
        if search::on_keydown(ke, &t) {
            return;
        }
        match cur_tab().as_str() {
            "cards" => cards::on_keydown(ke, &t),
            "lab" => lab::on_key(ke, &t, true),
            _ => {}
        }
    });
    listen(&d, "keyup", false, |e| {
        if let Some(t) = target(&e) {
            if cur_tab() == "lab" {
                lab::on_key(e.unchecked_ref(), &t, false);
            }
        }
    });
    for ev in ["pointerdown", "pointerup", "pointerleave", "pointercancel"] {
        let kind = ev.to_string();
        listen(&d, ev, true, move |e| {
            if cur_tab() != "lab" {
                return;
            }
            if let Some(t) = target(&e) {
                lab::on_pointer(&e, &t, &kind);
            }
        });
    }
    // toggle and element scroll do not bubble: listen in the capture phase
    listen(&d, "toggle", true, |e| {
        if let Some(t) = target(&e) {
            match cur_tab().as_str() {
                "learn" => learn::on_toggle(&t),
                "formulas" => formulas::on_toggle(&t),
                _ => {}
            }
        }
    });
    listen(&d, "scroll", true, |e| {
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            if has_class(&t, "side") && app().contains(Some(&t)) {
                learn::on_side_scroll(&t);
            }
        }
    });
}
