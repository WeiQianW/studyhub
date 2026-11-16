//! Calculators tab — every calculator's arithmetic is implemented here.
use crate::data::CalcMeta;
use crate::util::*;
use crate::*;
use std::collections::HashMap;
use std::f64::consts::{PI, SQRT_2};
use web_sys::Element;

const R3: f64 = 1.7320508075688772; // Math.sqrt(3)

struct V {
    n: HashMap<String, f64>,
    s: HashMap<String, String>,
}
impl V {
    fn f(&self, k: &str) -> f64 {
        *self.n.get(k).unwrap_or(&f64::NAN)
    }
    fn s(&self, k: &str) -> &str {
        self.s.get(k).map(|x| x.as_str()).unwrap_or("")
    }
}
type Out = Vec<(String, String)>;
fn row(k: &str, v: String) -> (String, String) {
    (k.to_string(), v)
}
fn err(m: &str) -> Out {
    vec![row("err", m.to_string())]
}
/// String(s).split(/[ ,]+/).map(Number)
fn nums(s: &str) -> Vec<f64> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut in_sep = false;
    for ch in s.chars() {
        if ch == ' ' || ch == ',' {
            if !in_sep {
                out.push(js_number(&cur));
                cur.clear();
                in_sep = true;
            }
        } else {
            in_sep = false;
            cur.push(ch);
        }
    }
    out.push(js_number(&cur));
    out
}

const E12: [f64; 12] = [1.0, 1.2, 1.5, 1.8, 2.2, 2.7, 3.3, 3.9, 4.7, 5.6, 6.8, 8.2];
const BANDS: [&str; 10] = ["black", "brown", "red", "orange", "yellow", "green", "blue", "violet", "grey", "white"];
fn e12_up(r: f64) -> f64 {
    let mut d = js_sys::Math::pow(10.0, r.log10().floor());
    for _ in 0..3 {
        for m in E12 {
            if m * d >= r * 0.9999 {
                return m * d;
            }
        }
        d *= 10.0;
    }
    r
}
fn colour_code(r: f64) -> String {
    let s = num_str(js_sys::Math::round(r));
    let b: Vec<char> = s.chars().collect();
    if b.len() < 2 {
        return "—".into();
    }
    let n = b.len() - 2;
    let band = |i: usize| BANDS.get(i).copied().unwrap_or("undefined");
    let dg = |c: char| c.to_digit(10).map(|d| d as usize).unwrap_or(99);
    format!("{} · {} · {}", band(dg(b[0])), band(dg(b[1])), band(n))
}

fn run(id: &str, v: &V, seg: &str) -> Out {
    match id {
        "c3p" => {
            let star = seg == "Star";
            let (vl, vp, il, ip);
            if v.s("vt") == "Line" {
                vl = v.f("V");
                vp = if star { vl / R3 } else { vl };
            } else {
                vp = v.f("V");
                vl = if star { vp * R3 } else { vp };
            }
            if v.s("it") == "Line" {
                il = v.f("I");
                ip = if star { il } else { il / R3 };
            } else {
                ip = v.f("I");
                il = if star { ip } else { ip * R3 };
            }
            let p = R3 * vl * il * v.f("pf");
            vec![
                row("Line voltage VL", fmt(vl, 2) + " V"),
                row("Phase voltage VP", fmt(vp, 2) + " V"),
                row("Line current IL", fmt(il, 3) + " A"),
                row("Phase current IP", fmt(ip, 3) + " A"),
                row("Power P", fmt(p / 1000.0, 3) + " kW"),
            ]
        }
        "cres" => {
            let star = seg == "Star";
            let vp = if star { v.f("VL") / R3 } else { v.f("VL") };
            let ip = vp / v.f("R");
            let il = if star { ip } else { ip * R3 };
            vec![
                row("Phase voltage", fmt(vp, 2) + " V"),
                row("Phase current", fmt(ip, 3) + " A"),
                row("Line current", fmt(il, 3) + " A"),
                row("Total power", fmt(3.0 * vp * ip / 1000.0, 3) + " kW"),
            ]
        }
        "ceff" => {
            let pi = if v.f("Pi") > 0.0 { v.f("Pi") } else { v.f("V") * v.f("I") };
            let po = if v.f("Po") > 0.0 { v.f("Po") } else { 2.0 * PI * v.f("N") * v.f("T") / 60.0 };
            let t = if v.f("Po") > 0.0 { 60.0 * po / (2.0 * PI * v.f("N")) } else { v.f("T") };
            vec![
                row("Input power", fmt(pi, 1) + " W"),
                row("Output power", fmt(po, 1) + " W"),
                row("Efficiency η", fmt(po / pi * 100.0, 1) + " %"),
                row("Torque", fmt(t, 2) + " N·m"),
                row("Output in HP", fmt(po / 746.0, 2) + " HP"),
            ]
        }
        "csync" => {
            let p = js_number(v.s("poles")) / 2.0;
            let n = 60.0 * v.f("f") / p;
            let nr = if v.f("Nr") > 0.0 { v.f("Nr") } else { n * (1.0 - v.f("s") / 100.0) };
            let s = (n - nr) / n;
            if nr > n {
                return err(&format!("Rotor speed cannot exceed synchronous speed ({} rpm) when motoring.", fmt(n, 2)));
            }
            vec![
                row("Pole pairs p", num_str(p)),
                row("Synchronous speed N", fmt(n, 2) + " rpm"),
                row("Rotor speed Nr", fmt(nr, 1) + " rpm"),
                row("Slip speed", fmt(n - nr, 1) + " rpm"),
                row("Slip s", fmt(s * 100.0, 2) + " %"),
                row("Rotor frequency fr", fmt(s * v.f("f"), 2) + " Hz"),
            ]
        }
        "cdc" => {
            if seg == "Shunt" {
                let i_f = v.f("V") / v.f("Rx");
                let ia = v.f("I") - i_f;
                if ia < 0.0 {
                    return err("Supply current is smaller than the field current.");
                }
                let eb = v.f("V") - ia * v.f("Ra");
                return vec![
                    row("Field current If", fmt(i_f, 3) + " A"),
                    row("Armature current Ia", fmt(ia, 3) + " A"),
                    row("Back EMF Eb", fmt(eb, 2) + " V"),
                    row("Input power", fmt(v.f("V") * v.f("I") / 1000.0, 3) + " kW"),
                ];
            }
            let eb = v.f("V") - v.f("I") * (v.f("Ra") + v.f("Rx"));
            vec![
                row("Ia = I", fmt(v.f("I"), 2) + " A"),
                row("Series field drop", fmt(v.f("I") * v.f("Rx"), 2) + " V"),
                row("Back EMF Eb", fmt(eb, 2) + " V"),
                row("Input power", fmt(v.f("V") * v.f("I") / 1000.0, 3) + " kW"),
            ]
        }
        "cemf" => {
            let p = v.f("poles") / 2.0;
            let cc = if v.s("w") == "Lap" { v.f("poles") } else { 2.0 };
            let e = 2.0 * p * (v.f("phi") / 1000.0) * v.f("N") * v.f("Z") / (60.0 * cc);
            vec![row("Pole pairs P", num_str(p)), row("Parallel paths c", num_str(cc)), row("EMF", fmt(e, 2) + " V")]
        }
        "cchop" => {
            let t = v.f("Ton") + v.f("Toff");
            let a = v.f("Ton") / t;
            vec![
                row("Period T", fmt(t, 3) + " ms"),
                row("Chopping frequency", fmt(1000.0 / t, 1) + " Hz"),
                row("Duty cycle A", fmt(a * 100.0, 1) + " %"),
                row("Average output V0", fmt(a * v.f("Vs"), 2) + " V"),
            ]
        }
        "cstep" => {
            let ang = 360.0 / v.f("steps");
            vec![
                row("Step angle", fmt(ang, 3) + " °"),
                row("Rotation for pulses", fmt(v.f("pulses") * ang / v.f("ratio"), 2) + " °"),
                row("Motor speed", fmt(v.f("pps") / v.f("steps") * 60.0, 2) + " rpm"),
                row("Output speed after gear", fmt(v.f("pps") / v.f("steps") * 60.0 / v.f("ratio"), 2) + " rpm"),
            ]
        }
        "cservo" => {
            let pw = v.f("pw");
            if pw < 1.0 || pw > 2.0 {
                return err("The notes use 1.0–2.0 ms pulses (every 20 ms).");
            }
            let a = (pw - 1.5) * 180.0;
            vec![
                row("Shaft angle", format!("{}{} °", if a > 0.0 { "+" } else { "" }, fmt(a, 1))),
                row("Direction", (if a == 0.0 { "Neutral" } else if a > 0.0 { "Clockwise" } else { "Anticlockwise" }).into()),
                row("Duty cycle (20 ms frame)", fmt(pw / 20.0 * 100.0, 2) + " %"),
            ]
        }
        "cplcp" => {
            let sum = |s: &str| nums(s).into_iter().filter(|x| x.is_finite()).fold(0.0, |a, b| a + b);
            let a = sum(v.s("i5"));
            let b = sum(v.s("i24"));
            let p = a * 5.0 + b * 24.0;
            vec![
                row("Total 5 V current", fmt(a, 3) + " A"),
                row("Total 24 V current", fmt(b, 3) + " A"),
                row("5 V power", fmt(a * 5.0, 3) + " W"),
                row("24 V power", fmt(b * 24.0, 3) + " W"),
                row("Total power", fmt(p, 3) + " W"),
                row("PSU headroom", if p <= v.f("psu") { fmt(v.f("psu") - p, 2) + " W spare" } else { format!("OVER by {} W", fmt(p - v.f("psu"), 2)) }),
            ]
        }
        "cohm" => {
            let (mut vv, mut i, mut r) = (v.f("V"), v.f("I"), v.f("R"));
            let n = [vv, i, r].iter().filter(|x| **x > 0.0).count();
            if n < 2 {
                return err("Enter any two of V, I and R.");
            }
            if !(vv > 0.0) {
                vv = i * r;
            } else if !(i > 0.0) {
                i = vv / r;
            } else if !(r > 0.0) {
                r = vv / i;
            }
            vec![
                row("Voltage V", fmt(vv, 3) + " V"),
                row("Current I", if i < 1.0 { fmt(i * 1000.0, 3) + " mA" } else { fmt(i, 3) + " A" }),
                row("Resistance R", fmt(r, 2) + " Ω"),
                row("Power P = V × I", if vv * i < 1.0 { fmt(vv * i * 1000.0, 2) + " mW" } else { fmt(vv * i, 3) + " W" }),
            ]
        }
        "cser" => {
            let r: Vec<f64> = nums(v.s("rs")).into_iter().filter(|x| *x > 0.0).collect();
            if r.is_empty() {
                return err("Enter at least one resistance.");
            }
            let rs = r.iter().fold(0.0, |a, b| a + b);
            let rp = 1.0 / r.iter().fold(0.0, |a, b| a + 1.0 / b);
            let vv = v.f("V");
            vec![
                row("Series RT", fmt(rs, 2) + " Ω"),
                row("Series current", fmt(vv / rs * 1000.0, 3) + " mA"),
                row("Series drops", r.iter().map(|x| fmt(vv * x / rs, 2) + " V").collect::<Vec<_>>().join(", ")),
                row("Parallel RT", fmt(rp, 3) + " Ω"),
                row("Parallel branch currents", r.iter().map(|x| fmt(vv / x, 3) + " A").collect::<Vec<_>>().join(", ")),
                row("Parallel total current", fmt(vv / rp, 3) + " A"),
            ]
        }
        "crect" => {
            let vp = SQRT_2 * v.f("Vr");
            let half = v.s("type") == "Half-wave";
            let nd = if v.s("type") == "Full-wave bridge" { 2.0 } else { 1.0 };
            let x = vp - nd * v.f("vd");
            let vpk = if x.is_nan() { f64::NAN } else { x.max(0.0) };
            let avg = if half { vpk / PI } else { 2.0 * vpk / PI };
            vec![
                row("Peak input Vp", fmt(vp, 2) + " V"),
                row("Peak output", fmt(vpk, 2) + " V"),
                row("Average DC", fmt(avg, 2) + " V"),
                row("Output pulse frequency", fmt(if half { v.f("f") } else { 2.0 * v.f("f") }, 2) + " Hz"),
                row("Diodes", (if half { "1" } else if nd == 2.0 { "4" } else { "2 (centre-tapped transformer)" }).into()),
            ]
        }
        "czen" => {
            if v.f("Vin") <= v.f("Vz") {
                return err("VIN must be greater than VZ for the zener to regulate.");
            }
            let vr = v.f("Vin") - v.f("Vz");
            let ir = vr / v.f("R");
            let il = v.f("Vz") / v.f("RL");
            let iz = ir - il;
            if iz < 0.0 {
                return vec![
                    row("VR", fmt(vr, 2) + " V"),
                    row("IR", fmt(ir * 1000.0, 3) + " mA"),
                    row("IL wanted", fmt(il * 1000.0, 3) + " mA"),
                    row("err", "IZ would be negative — the load takes more than R can supply, so the output falls below VZ (not regulating).".into()),
                ];
            }
            vec![
                row("VL = VZ", fmt(v.f("Vz"), 2) + " V"),
                row("VR = VIN − VZ", fmt(vr, 2) + " V"),
                row("IR = VR / R", fmt(ir * 1000.0, 3) + " mA"),
                row("IL = VZ / RL", fmt(il * 1000.0, 3) + " mA"),
                row("IZ = IR − IL", fmt(iz * 1000.0, 3) + " mA"),
                row("Zener power", fmt(v.f("Vz") * iz * 1000.0, 2) + " mW"),
            ]
        }
        "cbjt" => {
            let ib = v.f("IB") / 1e6;
            let ic = v.f("beta") * ib;
            let ie = ic + ib;
            vec![
                row("IC = β × IB", fmt(ic * 1000.0, 3) + " mA"),
                row("IE = IC + IB", fmt(ie * 1000.0, 3) + " mA"),
                row("α = β / (1 + β)", fmt(v.f("beta") / (1.0 + v.f("beta")), 4)),
            ]
        }
        "cbase" => {
            let s = js_trim(v.s("n")).to_uppercase();
            let b = js_number(v.s("b")) as u32;
            let ok = !s.is_empty()
                && s.chars().all(|ch| match b {
                    2 => ch == '0' || ch == '1',
                    8 => ('0'..='7').contains(&ch),
                    10 => ch.is_ascii_digit(),
                    _ => ch.is_ascii_hexdigit(),
                });
            if !ok {
                return err(&format!("Digits not valid for base {}.", b));
            }
            let d = match u64::from_str_radix(&s, b) {
                Ok(d) if d <= 9007199254740991 => d,
                _ => return err("Number too large."),
            };
            let bin = format!("{:b}", d);
            let w = (bin.len() + 3) / 4 * 4;
            let padded = format!("{:0>w$}", bin, w = w);
            let g4 = padded.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(" ");
            let ds = d.to_string();
            vec![
                row("Decimal", ds.clone()),
                row("Binary", g4),
                row("Octal", format!("{:o}", d)),
                row("Hexadecimal", format!("{:X}", d)),
                row("BCD", ds.chars().map(|ch| format!("{:04b}", ch.to_digit(10).unwrap())).collect::<Vec<_>>().join(" ")),
                row("Bits needed", bin.len().to_string()),
            ]
        }
        "cwave" => {
            if v.f("tw") > v.f("T") {
                return err("The pulse width cannot be longer than the period.");
            }
            let f = 1000.0 / v.f("T");
            vec![
                row("Frequency f = 1/T", if f >= 1000.0 { fmt(f / 1000.0, 3) + " kHz" } else { fmt(f, 3) + " Hz" }),
                row("Duty cycle", fmt(v.f("tw") / v.f("T") * 100.0, 2) + " %"),
                row("LOW time", fmt(v.f("T") - v.f("tw"), 3) + " ms"),
            ]
        }
        "cspin" => {
            let mat = v.s("mat");
            let tab: (f64, f64) = match mat {
                "Mild steel" => (25.0, 30.0),
                "Cast iron" => (20.0, 30.0),
                "Brass" => (40.0, 60.0),
                _ => (60.0, 90.0),
            };
            let d = v.f("d");
            let n = |vv: f64| 1000.0 * vv / (PI * d);
            let mut out = vec![];
            let vv = v.f("V");
            if vv > 0.0 {
                out.push(row(&format!("Spindle speed at {} m/min", fmt(vv, 2)), fmt(n(vv), 2) + " rpm"));
                if vv < tab.0 || vv > tab.1 {
                    out.push(row("Note", format!("Outside the notes' range for {} ({}–{} m/min)", mat.to_lowercase(), num_str(tab.0), num_str(tab.1))));
                }
            }
            out.push(row(&format!("Table range {}–{} m/min", num_str(tab.0), num_str(tab.1)), format!("{} – {} rpm", fmt(n(tab.0), 2), fmt(n(tab.1), 2))));
            out
        }
        "ctap" => {
            if v.f("P") >= v.f("D") {
                return err("Pitch must be smaller than the diameter.");
            }
            let t = v.f("D") - v.f("P");
            vec![
                row("Thread", format!("M{} × {}", fmt(v.f("D"), 2), fmt(v.f("P"), 2))),
                row("Tap drill = D − P", fmt(t, 2) + " mm"),
                row("Countersink / deburr", "light chamfer at the hole mouth to help start the tap".into()),
            ]
        }
        "cgear" => {
            let n2 = v.f("N1") * v.f("a") / v.f("b");
            let mut out = vec![row("Stage 1 ratio", fmt(v.f("b") / v.f("a"), 3) + " : 1"), row("Speed after stage 1", fmt(n2, 2) + " rpm")];
            if v.f("c") > 0.0 && v.f("d") > 0.0 {
                let n4 = n2 * v.f("c") / v.f("d");
                out.push(row("Stage 2 ratio", fmt(v.f("d") / v.f("c"), 3) + " : 1"));
                out.push(row("Final speed (compound)", fmt(n4, 2) + " rpm"));
                out.push(row("Overall ratio", fmt(v.f("N1") / n4, 3) + " : 1"));
            }
            out.push(row("Direction (simple pair)", "reversed for gears; same for an open belt".into()));
            out
        }
        "cfit" => {
            if v.f("hh") < v.f("hl") || v.f("sh") < v.f("sl") {
                return err("Each high limit must be at least the low limit.");
            }
            let mx = v.f("hh") - v.f("sl");
            let mnn = v.f("hl") - v.f("sh");
            let s = |x: f64| format!("{}{} mm", if x >= 0.0 { "+" } else { "" }, fmt(x, 3));
            let ty = if mnn >= 0.0 { "Clearance fit" } else if mx <= 0.0 { "Interference fit" } else { "Transition fit" };
            vec![
                row("Hole tolerance", fmt(v.f("hh") - v.f("hl"), 3) + " mm"),
                row("Shaft tolerance", fmt(v.f("sh") - v.f("sl"), 3) + " mm"),
                row("Max allowance", s(mx)),
                row("Min allowance", s(mnn)),
                row("Type of fit", ty.into()),
            ]
        }
        "cpnf" => {
            if !(v.f("D") > 0.0) || !(v.f("P") > 0.0) {
                return err("Enter a bore diameter and a pressure above zero.");
            }
            if v.f("d") >= v.f("D") {
                return err("The rod must be smaller than the bore.");
            }
            let a_big = PI * js_sys::Math::pow(v.f("D") / 1000.0, 2.0) / 4.0;
            let a = PI * js_sys::Math::pow(v.f("d").max(0.0) / 1000.0, 2.0) / 4.0;
            let pa = v.f("P") * 1e5;
            let fe = pa * a_big;
            let fr = pa * (a_big - a);
            vec![
                row("Piston area A", fmt(a_big * 1e4, 3) + " cm²"),
                row("Rod area a", fmt(a * 1e4, 3) + " cm²"),
                row("Extend force P × A", format!("{} N ≈ {} kgf", fmt(fe, 1), fmt(fe / 9.81, 1))),
                row("Retract force P × (A − a)", format!("{} N ≈ {} kgf", fmt(fr, 1), fmt(fr / 9.81, 1))),
                row("Absolute pressure", fmt(v.f("P") + 1.013, 3) + " bar abs"),
            ]
        }
        "cled" => {
            if !(v.f("I") > 0.0) {
                return err("Enter an LED current above zero.");
            }
            if v.f("Vf") >= v.f("Vs") {
                return err("The supply must be higher than the LED forward voltage.");
            }
            let r = (v.f("Vs") - v.f("Vf")) / (v.f("I") / 1000.0);
            let rs = e12_up(r);
            let i_n = (v.f("Vs") - v.f("Vf")) / rs;
            vec![
                row("R = (Vs − Vf) / I", fmt(r, 1) + " Ω"),
                row("Next standard (E12) value", fmt(rs, 0) + " Ω"),
                row("Colour bands", colour_code(rs)),
                row("Current with that resistor", fmt(i_n * 1000.0, 2) + " mA"),
                row("Resistor power", fmt((v.f("Vs") - v.f("Vf")) * i_n * 1000.0, 1) + " mW"),
            ]
        }
        "cus" => {
            let x = v.f("x");
            if !(x > 0.0) {
                return err("Enter a value above zero.");
            }
            if v.s("mode") == "Echo time → distance" {
                let cm = x / 29.0 / 2.0;
                return vec![
                    row("Distance = t / 29 / 2", fmt(cm, 1) + " cm"),
                    row("int distance (as the Arduino computes it)", format!("{} cm", num_str(((x / 29.0).floor() / 2.0).floor()))),
                    row("Round-trip path", fmt(cm * 2.0, 1) + " cm"),
                ];
            }
            let t = x * 2.0 * 29.0;
            vec![row("Echo time = d × 2 × 29", fmt(t, 0) + " µs"), row("In milliseconds", fmt(t / 1000.0, 3) + " ms")]
        }
        "cpwm" => {
            let x = v.f("x");
            let (val, duty);
            if v.s("mode") == "Duty cycle (%)" {
                if x < 0.0 || x > 100.0 {
                    return err("Duty cycle must be 0–100 %.");
                }
                duty = x / 100.0;
                val = js_sys::Math::round(duty * 255.0);
            } else {
                if x < 0.0 || x > 255.0 {
                    return err("analogWrite takes 0–255.");
                }
                val = js_sys::Math::round(x);
                duty = val / 255.0;
            }
            vec![row("analogWrite value", num_str(val)), row("Duty cycle", fmt(duty * 100.0, 1) + " %"), row("Average voltage", fmt(duty * v.f("V"), 2) + " V")]
        }
        "cpay" => {
            if !(v.f("P") > 0.0) {
                return err("Enter the rated payload.");
            }
            let left = v.f("P") - v.f("T");
            if left <= 0.0 {
                return err("The tooling alone reaches the rated payload — no capacity left for a part.");
            }
            let mut out = vec![row("Maximum part mass = payload − tooling", fmt(left, 2) + " kg")];
            if v.f("W") > 0.0 {
                let tot = v.f("T") + v.f("W");
                out.push(row("Tooling + part", fmt(tot, 2) + " kg"));
                out.push(row(
                    "Result",
                    if tot <= v.f("P") {
                        format!("within payload ({} % used)", fmt(tot / v.f("P") * 100.0, 0))
                    } else {
                        format!("OVER payload by {} kg", fmt(tot - v.f("P"), 2))
                    },
                ));
            }
            out
        }
        _ => vec![],
    }
}

fn is_text(cm: &CalcMeta, k: &str) -> bool {
    cm.text.as_ref().map(|t| t.iter().any(|x| x == k)).unwrap_or(false)
}

pub fn render() {
    let c = c();
    with(|s| s.calc_seg.clear());
    let mod_of = |cm: &CalcMeta| c.topic(&cm.topic).map(|t| t.module.clone()).unwrap_or_default();
    let mut h = String::from("<div class=\"toolbar\"><h2 style=\"margin-right:auto\">Calculators</h2><span class=\"muted\" style=\"font-size:13px\">Pre-filled with examples from the notes — change any value.</span></div>");
    for m in c.mod_order.iter().filter(|m| c.calcs.iter().any(|cm| mod_of(cm) == **m)) {
        h.push_str(&format!("<h3 class=\"fxg\">{} · {}</h3><div class=\"calcgrid\">", c.mods[m].code, mn(m)));
        for cm in c.calcs.iter().filter(|cm| mod_of(cm) == *m) {
            let seg = cm
                .seg
                .as_ref()
                .map(|sg| {
                    format!(
                        "<div class=\"seg\">{}</div>",
                        sg.iter().enumerate().map(|(i, s)| format!("<button data-seg=\"{}\" aria-pressed=\"{}\">{}</button>", s, i == 0, s)).collect::<String>()
                    )
                })
                .unwrap_or_default();
            let fields: String = cm
                .fields
                .iter()
                .map(|f| {
                    let k = f[0].as_str().unwrap_or("");
                    let lb = f[1].as_str().unwrap_or("");
                    let ctl = match &f[2] {
                        serde_json::Value::Array(opts) => format!(
                            "<select id=\"{}-{}\">{}</select>",
                            cm.id,
                            k,
                            opts.iter()
                                .map(|o| {
                                    let o = js_string(o);
                                    format!("<option value=\"{}\" {}>{}</option>", o, if cm.id == "csync" && o == "2" { "selected" } else { "" }, o)
                                })
                                .collect::<String>()
                        ),
                        d => format!(
                            "<input id=\"{}-{}\" {} value=\"{}\">",
                            cm.id,
                            k,
                            if is_text(cm, k) { "type=\"text\"" } else { "type=\"number\" step=\"any\" inputmode=\"decimal\"" },
                            js_string(d)
                        ),
                    };
                    format!("<div class=\"field\"><label for=\"{}-{}\">{}</label>{}</div>", cm.id, k, lb, ctl)
                })
                .collect();
            h.push_str(&format!(
                "\n  <div class=\"card calc\" id=\"{id}\"><div style=\"display:flex;justify-content:space-between;gap:8px;align-items:baseline;flex-wrap:wrap\"><h3>{title}</h3><button class=\"chipbtn\" data-t=\"{topic}\">{wk} notes</button></div>\n  {seg}\n  <div class=\"row\">{fields}</div>\n  <dl class=\"out\" aria-live=\"polite\"></dl></div>",
                id = cm.id,
                title = cm.title,
                topic = cm.topic,
                wk = c.topic(&cm.topic).map(|t| t.wk.clone()).unwrap_or_default(),
                seg = seg,
                fields = fields
            ));
        }
        h.push_str("</div>");
    }
    set_html(&app(), &h);
    for cm in &c.calcs {
        compute(cm);
    }
}

fn compute(cm: &CalcMeta) {
    let el = match by_id(&cm.id) {
        Some(e) => e,
        None => return,
    };
    let mut v = V { n: HashMap::new(), s: HashMap::new() };
    let mut bad = false;
    for f in &cm.fields {
        let k = f[0].as_str().unwrap_or("");
        let i = by_id(&format!("{}-{}", cm.id, k)).unwrap();
        let val = input_value(&i);
        if f[2].is_array() || is_text(cm, k) {
            v.s.insert(k.into(), val);
        } else {
            let x = parse_float(&val);
            if !x.is_finite() {
                bad = true;
            }
            v.n.insert(k.into(), x);
        }
    }
    let out = match qs(&el, ".out") {
        Some(o) => o,
        None => return,
    };
    if bad {
        set_html(&out, "<div class=\"err\">Enter a number in every field.</div>");
        return;
    }
    let seg = with(|s| s.calc_seg.get(&cm.id).cloned()).or_else(|| cm.seg.as_ref().map(|s| s[0].clone())).unwrap_or_default();
    let res = run(&cm.id, &v, &seg);
    let h: String = res
        .iter()
        .map(|(k, val)| if k == "err" { format!("<div class=\"err\">{}</div>", val) } else { format!("<dt>{}</dt><dd>{}</dd>", k, val) })
        .collect();
    set_html(&out, &h);
}

pub fn on_input(t: &Element) {
    if let Some(card) = closest(t, "#app .card.calc") {
        if let Some(cm) = c().calcs.iter().find(|x| x.id == card.id()) {
            compute(cm);
        }
    }
}
pub fn on_click(t: &Element) -> bool {
    if let Some(b) = closest(t, "#app [data-t]") {
        set_topic(&attr(&b, "data-t"), false);
        go("learn", GoOpts::default());
        return true;
    }
    if let Some(b) = closest(t, "#app [data-seg]") {
        let card = match closest(&b, ".card.calc") {
            Some(c) => c,
            None => return true,
        };
        let id = card.id();
        let seg = attr(&b, "data-seg");
        with(|s| s.calc_seg.insert(id.clone(), seg.clone()));
        for x in qsa(&card, "[data-seg]") {
            set_attr(&x, "aria-pressed", if x.is_same_node(Some(&b)) { "true" } else { "false" });
        }
        if id == "cdc" {
            let sh = seg == "Shunt";
            let setv = |k: &str, v: &str| {
                if let Some(e) = by_id(k) {
                    set_input_value(&e, v);
                }
            };
            setv("cdc-Rx", if sh { "200" } else { "0.3" });
            setv("cdc-V", if sh { "440" } else { "200" });
            setv("cdc-I", if sh { "19.95" } else { "45" });
            setv("cdc-Ra", if sh { "0.8" } else { "0.5" });
        }
        if let Some(cm) = c().calcs.iter().find(|x| x.id == id) {
            compute(cm);
        }
        return true;
    }
    false
}
