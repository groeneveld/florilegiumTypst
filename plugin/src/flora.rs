// flora.rs -- the primitives of florilegium, drawn with ink.rs (a port of flora.lua).
//
// Every primitive draws in its own local frame (centimeters, origin at its base
// or center, pointing along +y unless noted) under the current ink transform,
// and takes an rng so that no two instances are alike. Painting is layered like
// a watercolor: a flat wash, darker blotches multiplied over it, pooling at the
// edges, pale streaks lifted out, a few fine ink lines on top. There are almost
// no outlines; edges are defined by the wash.

use crate::ink::{self, fmod, lerp, tone, Color, Pt, Rng, CM, RAD, WHITE};
use std::f64::consts::PI;

// the options of all primitives; each reads the ones it knows
#[derive(Clone, Default)]
pub struct O {
    pub w: Option<f64>,
    pub bend: Option<f64>,
    pub tone: Option<f64>,
    pub streaks: Option<i64>,
    pub pointed: bool,
    pub heart: Option<bool>,
    pub petals: Option<i64>,
    pub tilt: Option<f64>,
    pub eye: Option<Color>,
    pub star: Option<bool>,
    pub side: Option<f64>,
    pub fork: Option<bool>,
    pub depth: Option<f64>,
    pub lobes: Option<i64>,
    pub open: Option<f64>,
    pub trail: Option<bool>,
    pub facing: Option<f64>,
    pub drift: Option<i64>,
    pub color: Option<Color>,
    pub rays: Option<bool>,
}

fn d() -> O {
    O::default()
}

// level of detail: how many centimeters one local unit covers on the page,
// and whether we are drawing a pale ghost (no fine detail then)
fn lod() -> f64 {
    ink::unit()
}
fn ghost_on() -> bool {
    ink::get_lift() > 0.3
}
fn sgn(b: bool) -> f64 {
    if b { 1.0 } else { -1.0 }
}
fn sin(x: f64) -> f64 {
    x.sin()
}

// ---------------------------------------------------------------- washes
fn speck(x: f64, y: f64, r: f64, a: f64) {
    let (c, s) = (a.cos(), a.sin());
    ink::poly(&[[x + r * c, y + r * s], [x - 0.6 * r * s, y + 0.6 * r * c], [x - r * c, y - r * s], [x + 0.6 * r * s, y - 0.6 * r * c]], true);
}

// Granulated watercolor inside the current clip: darker and lighter blotches
// over a box, then fine specks. `n` blotches of about `size`; `dark`/`light` tones.
#[allow(clippy::too_many_arguments)]
pub fn mottle(rng: &Rng, x0: f64, y0: f64, x1: f64, y1: f64, n: usize, size: f64, dark: f64, light: f64, specks: usize) {
    for _ in 0..n {
        let (x, y) = (rng.range(x0, x1), rng.range(y0, y1));
        let r = size * rng.range(0.5, 1.4);
        let ry = r * rng.range(0.5, 1.0);
        ink::smooth(&ink::blob(rng, x, y, r, ry, 0.35, 7), true);
        if rng.chance(0.6) {
            ink::fill(tone(dark + rng.sym(0.06)), rng.range(0.10, 0.22), true)
        } else {
            ink::fill(tone(light + rng.sym(0.06)), rng.range(0.12, 0.26), false)
        }
    }
    if specks > 0 && !ghost_on() {
        for _ in 0..specks {
            speck(rng.range(x0, x1), rng.range(y0, y1), size * rng.range(0.03, 0.08), rng.range(0.0, PI));
        }
        ink::fill(tone(dark - 0.08), rng.range(0.3, 0.5), true);
    }
}

// a tapered ribbon along a curve f(v), widths w0 at the start and w1 at the end
pub fn streak(f: ink::Spine, n: usize, w0: f64, w1: f64, c: Color, op: f64, mult: bool) {
    let pts = ink::sample(f, n);
    ink::smooth(&ink::ribbon(&pts, &|u| lerp(w0, w1, u) * if u < 0.12 { 0.4 + 5.0 * u } else { 1.0 }), true);
    ink::fill(c, op, mult);
}

// the same along one cubic Bézier (control points), much cheaper
#[allow(clippy::too_many_arguments)]
pub fn streak3(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, w0: f64, w1: f64, c: Color, op: f64, mult: bool) {
    ink::taper(x0, y0, x1, y1, x2, y2, x3, y3, w0, w1);
    ink::fill(c, op, mult);
}

// ---------------------------------------------------------------- petal
// half-width of an obovate petal: narrow claw, widest at 0.62, elliptical cap
fn round_half(w: f64) -> impl Fn(f64) -> f64 {
    move |v| {
        let m = 0.62;
        if v < m {
            return w * (0.16 + 0.84 * sin(PI / 2.0 * v / m).powf(1.3));
        }
        let q = (v - m) / (1.0 - m);
        w * (1.0 - q * q).max(0.0).sqrt()
    }
}

// Base at the origin, pointing +y, length 1.
//   round:   obovate, narrow claw, elliptical cap, wavy margin, maybe a notch
//   pointed: lanceolate, for lilies and stars
// Opts: w (half-width), tone (wash), streaks, bend (tip offset), pointed, heart (pale base)
pub fn petal(rng: &Rng, o: &O) -> Vec<Pt> {
    ink::in_bloom(|| petal_(rng, o))
}
fn petal_(rng: &Rng, o: &O) -> Vec<Pt> {
    let pointed = o.pointed;
    let w = o.w.unwrap_or_else(|| if pointed { rng.range(0.15, 0.2) } else { rng.range(0.40, 0.48) });
    let bend = o.bend.unwrap_or_else(|| rng.sym(0.08));
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.34, 0.42));
    let u = lod();
    let fine = u > 0.45 && !ghost_on();
    let wave = ink::fbm1(rng, 3);
    let notch = if !pointed && rng.chance(0.4) { rng.range(0.03, 0.06) } else { 0.0 };
    let spine = |v: f64| (bend * v * v, v);
    let rh = round_half(w);
    let half = |v: f64| {
        if pointed { w * 2.0 * sin(PI * v.powf(0.8)).powf(0.85) * (1.0 - 0.3 * v) } else { rh(v) }
    };
    let edge = |v: f64, side: f64| {
        let mut e = (if pointed { 0.012 } else { 0.03 }) * wave.at(v * 6.0 + if side > 0.0 { 0.0 } else { 40.0 }) * v;
        if notch > 0.0 && v > 0.9 {
            e -= notch * (v - 0.9) * 10.0 * (1.0 - (v - 0.9) * 10.0);
        }
        e
    };
    let (outline, _, _) = ink::blade(&spine, if u > 0.5 { 28 } else { 18 }, &half, &half, Some(&edge));
    ink::smooth(&outline, true);
    ink::clip();
    ink::flood(tone(t0), -1.0, -0.1, 1.0, 1.2);
    if o.heart != Some(false) {
        // the heart of the flower is pale
        for k in 1..=8 {
            let k = k as f64;
            ink::smooth(&ink::blob(rng, 0.0, 0.0, w * (0.25 + 0.1 * k), 0.06 + 0.065 * k, 0.28, 9), true);
            ink::fill(WHITE, 0.1, false);
        }
    }
    mottle(rng, -w * 1.2, 0.12, w * 1.2, 1.05, if fine { 15 } else { 6 }, if pointed { 0.06 } else { 0.12 }, t0 - 0.2, t0 + 0.2, if fine { 12 } else { 0 });
    // darker brush strokes running out from the base, between the streaks
    if fine {
        for _ in 0..rng.int(5, 8) {
            let f = rng.sym(0.95);
            let (v0, v1) = (rng.range(0.15, 0.4), rng.range(0.7, 1.0));
            let (x0, x1) = (f * half(v0) * 0.8 + bend * v0 * v0, f * half(v1) * 0.9 + bend * v1 * v1);
            let w0 = rng.range(0.03, 0.07);
            let op = rng.range(0.12, 0.25);
            streak3(x0, v0, x0 + (x1 - x0) * 0.3, lerp(v0, v1, 0.35), x1 * 0.95, lerp(v0, v1, 0.7), x1, v1, w0, 0.005, tone(t0 - 0.25), op, true);
        }
    }
    // pooling: the outline restroked wide and soft, only its inner half shows
    ink::smooth(&outline, true);
    ink::stroke(tone(0.08), 1.6 * u, if ghost_on() { 0.15 } else { 0.3 }, true);
    // white streaks lifted out, fanning from the base
    let ns = o.streaks.unwrap_or_else(|| if pointed { rng.int(3, 5) } else { rng.int(9, 13) });
    for k in 1..=ns {
        let mut f = (k as f64 - 0.5) / ns as f64 * 2.0 - 1.0 + rng.sym(0.05);
        let mut len = (if pointed { rng.range(0.4, 0.85) } else { rng.range(0.35, 0.8) }) * (1.0 - 0.35 * f * f);
        if pointed && k == (ns + 1) / 2 {
            f = 0.0;
            len = rng.range(0.8, 0.92);
        }
        let x1 = f * half(len) * 0.85 + bend * len * len;
        let mx = f * half(len * 0.5) * 0.6 + rng.sym(0.02) + bend * len * len * 0.3;
        let w0 = (if pointed { 0.6 } else { 1.0 }) * rng.range(0.022, 0.045);
        let op = rng.range(0.55, 0.9);
        streak3(f * 0.03, 0.0, mx * 0.5, len * 0.3, mx, len * 0.6, x1, len, w0, 0.002, WHITE, op, false);
    }
    // a few fine dark veins, following the petal
    if fine {
        for _ in 0..rng.int(4, 7) {
            let f = rng.sym(0.85);
            let (v0, v1) = (rng.range(0.2, 0.35), rng.range(0.75, 0.95));
            let (x0, x1) = (f * half(v0) * 0.7 + bend * v0 * v0, f * half(v1) * 0.9 + bend * v1 * v1);
            ink::smooth(&ink::sample(&ink::bezier3(x0, v0, x0 + (x1 - x0) * 0.2, lerp(v0, v1, 0.4), x1 * 0.97, lerp(v0, v1, 0.75), x1, v1), 5), false);
            ink::stroke(tone(0.1), 0.15, rng.range(0.18, 0.32), true);
        }
    }
    ink::unclip();
    outline
}

// ---------------------------------------------------------------- flowers
// the white star-shaped heart, stamens and a copper eye, at the origin; r = radius scale
fn heart(rng: &Rng, np: i64, r: f64) {
    for k in 1..=2 * np {
        let a = (k as f64 - 0.5) * 360.0 / (2 * np) as f64 + rng.sym(8.0);
        ink::push();
        ink::rotate(a);
        streak3(0.0, 0.0, 0.01, 0.1 * r, -0.01, 0.2 * r, 0.0, rng.range(0.24, 0.36) * r, 0.06 * r, 0.004 * r, WHITE, 0.85, false);
        ink::pop();
    }
    ink::circle(0.0, 0.0, 0.13 * r);
    ink::fill(WHITE, 0.9, false);
    if !ghost_on() {
        for _ in 0..14 {
            let (a, d) = (rng.range(0.0, 2.0 * PI), rng.range(0.05, 0.12) * r);
            ink::moveto(0.0, 0.0);
            ink::lineto(d * a.cos(), d * a.sin());
            ink::stroke(tone(0.1), 0.2, 0.7, false);
            ink::circle(d * a.cos(), d * a.sin(), 0.008 * r);
            ink::fill(tone(0.05), 0.9, false);
        }
    }
    ink::circle(0.0, 0.0, 0.042 * r);
    ink::fill1(ink::accent(-0.15));
    ink::circle(0.0, 0.0, 0.022 * r);
    ink::fill1(ink::accent(0.3));
}

// Five broad petals seen from the front. s = petal length (cm); center at the origin.
pub fn flower(rng: &Rng, s: f64, o: &O) {
    ink::in_bloom(|| flower_(rng, s, o))
}
fn flower_(rng: &Rng, s: f64, o: &O) {
    ink::push();
    ink::scale1(s);
    ink::rotate(rng.range(0.0, 72.0));
    let np = o.petals.unwrap_or(5);
    let t0 = o.tone.unwrap_or_else(|| if rng.chance(0.25) { rng.range(0.2, 0.28) } else { rng.range(0.3, 0.46) });
    for k in 0..np {
        ink::push();
        ink::rotate(k as f64 * 360.0 / np as f64 + rng.sym(6.0));
        ink::scale1(rng.range(0.9, 1.06));
        ink::translate(0.0, 0.04);
        petal(rng, &O { tone: Some(t0 + rng.sym(0.04)), ..d() });
        ink::pop();
    }
    heart(rng, np, 1.0);
    ink::pop();
}

// A star lily seen at a tilt: six pointed, recurved petals in a star that is
// squashed along y, so the petals behind (pointing up) are short and darker and
// the ones in front are long and pale; stamens stand out of the throat.
// Center at the origin, the flower faces +y; s = petal length (cm).
pub fn lily(rng: &Rng, s: f64, o: &O) {
    ink::in_bloom(|| lily_(rng, s, o))
}
fn lily_(rng: &Rng, s: f64, o: &O) {
    ink::push();
    ink::scale1(s);
    let tilt = o.tilt.unwrap_or_else(|| rng.range(0.45, 0.62));
    let spin = rng.range(0.0, 60.0);
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.30, 0.38));
    let mut order: Vec<f64> = (0..6).map(|k| spin + k as f64 * 60.0 + rng.sym(8.0)).collect();
    // back petals first
    order.sort_by(|a, b| (b * RAD).sin().partial_cmp(&(a * RAD).sin()).unwrap_or(std::cmp::Ordering::Equal));
    for a in order {
        let back = (a * RAD).sin();
        ink::push();
        ink::scale(1.0, tilt);
        ink::rotate(a - 90.0);
        ink::scale1(rng.range(0.88, 1.05));
        let w = rng.range(0.11, 0.14);
        let tn = t0 + 0.1 * back * -0.8 + rng.sym(0.03);
        let bend = sgn(rng.chance(0.5)) * rng.range(0.04, 0.14);
        let streaks = rng.int(3, 4);
        petal(rng, &O { pointed: true, w: Some(w), tone: Some(tn), bend: Some(bend), heart: Some(false), streaks: Some(streaks), ..d() });
        ink::pop();
    }
    // pale throat
    for k in 1..=4 {
        let r = 0.05 + 0.05 * k as f64;
        ink::smooth(&ink::blob(rng, 0.0, 0.0, r, r * tilt, 0.2, 9), true);
        ink::fill(WHITE, 0.22, false);
    }
    // stamens: curved filaments, dark anthers, copper pollen
    if !ghost_on() {
        for k in 1..=6 {
            let a = (k as f64 - 3.5) * 16.0 + rng.sym(6.0);
            let l = rng.range(0.3, 0.48);
            let (ex, ey) = (l * (a * RAD).sin(), l * (a * RAD).cos());
            ink::smooth(&ink::sample(&ink::bezier3(0.0, 0.0, ex * 0.2, ey * 0.5, ex * 0.8, ey * 0.85, ex, ey), 6), false);
            ink::stroke(tone(0.12), 0.25, 0.85, false);
            ink::push();
            ink::translate(ex, ey);
            ink::rotate(-a + rng.sym(30.0));
            ink::smooth(&ink::blob(rng, 0.0, 0.0, 0.022, 0.04, 0.1, 6), true);
            ink::fill1(tone(0.05));
            ink::pop();
            if rng.chance(0.5) {
                ink::circle(ex + rng.sym(0.03), ey + rng.range(0.02, 0.05), 0.008);
                ink::fill1(ink::accent(0.0));
            }
        }
    }
    ink::circle(0.0, 0.0, 0.035);
    ink::fill1(ink::accent(-0.2));
    ink::pop();
}

// A small flower with round petals, seen from the front: violets, forget-me-nots.
// r = radius (cm). Fewer layers than the large flower.
pub fn blossom(rng: &Rng, r: f64, o: &O) {
    ink::in_bloom(|| blossom_(rng, r, o))
}
fn blossom_(rng: &Rng, r: f64, o: &O) {
    ink::push();
    ink::scale1(r);
    ink::rotate(rng.range(0.0, 72.0));
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.30, 0.45));
    for k in 0..5 {
        ink::push();
        ink::rotate(k as f64 * 72.0 + rng.sym(8.0));
        ink::scale1(rng.range(0.9, 1.08));
        let hw = round_half(rng.range(0.36, 0.44));
        let (outline, _, _) = ink::blade(&|v| (0.0, v), 18, &hw, &hw, None);
        ink::smooth(&outline, true);
        ink::fill1(tone(t0 + rng.sym(0.05)));
        ink::smooth(&outline, true);
        ink::clip();
        let (bx, by) = (rng.sym(0.2), rng.range(0.5, 0.9));
        ink::smooth(&ink::blob(rng, bx, by, 0.35, 0.3, 0.3, 8), true);
        ink::fill(tone(t0 - 0.2), 0.3, true);
        streak3(0.0, 0.0, 0.02, 0.2, -0.02, 0.4, rng.sym(0.05), 0.55, 0.12, 0.01, WHITE, 0.7, false);
        ink::smooth(&outline, true);
        ink::stroke(tone(0.08), 1.2 * lod(), 0.25, true);
        ink::unclip();
        ink::pop();
    }
    ink::circle(0.0, 0.0, 0.22);
    ink::fill(WHITE, 0.85, false);
    ink::circle(0.0, 0.0, 0.09);
    ink::fill1(o.eye.unwrap_or_else(|| tone(0.08)));
    ink::pop();
}

// A floret a few millimeters across: five petals, round (forget-me-not) or
// pointed (star), one fill, a dark eye. r = radius (cm). Cheap: used by the hundred.
pub fn floret(rng: &Rng, r: f64, o: &O) {
    ink::in_bloom(|| floret_(rng, r, o))
}
fn floret_(rng: &Rng, r: f64, o: &O) {
    let star = o.star.unwrap_or_else(|| rng.chance(0.5));
    if r * lod() < 0.07 {
        let ph = rng.range(0.0, 2.0 * PI);
        let pts: Vec<Pt> = (0..10)
            .map(|k| {
                let a = ph + k as f64 * PI / 5.0;
                let d = if k % 2 == 0 { r } else { r * if star { 0.32 } else { 0.62 } };
                [d * a.cos(), d * a.sin()]
            })
            .collect();
        ink::poly(&pts, true);
        ink::fill1(tone(o.tone.unwrap_or_else(|| rng.range(0.22, 0.42))));
        let e = 0.2 * r;
        ink::poly(&[[e, 0.0], [0.0, e], [-e, 0.0], [0.0, -e]], true);
        ink::fill1(o.eye.unwrap_or_else(|| tone(0.05)));
        return;
    }
    let ph = rng.range(0.0, 2.0 * PI);
    for k in 0..5 {
        let a = ph + k as f64 * 2.0 * PI / 5.0 + rng.sym(0.15);
        let (c, s) = (a.cos(), a.sin());
        let l = r * rng.range(0.85, 1.1);
        let w = if star { 0.26 } else { 0.42 };
        let pts: Vec<Pt> = if star {
            vec![[0.0, 0.0], [0.45 * l, w * l * 0.6], [l, 0.0], [0.45 * l, -w * l * 0.6]]
        } else {
            vec![[0.0, 0.0], [0.3 * l, w * l], [0.85 * l, w * l * 0.8], [l, 0.0], [0.85 * l, -w * l * 0.8], [0.3 * l, -w * l]]
        };
        let pts: Vec<Pt> = pts.iter().map(|p| [p[0] * c - p[1] * s, p[0] * s + p[1] * c]).collect();
        ink::smooth_t(&pts, true, if star { 0.5 } else { 1.0 });
    }
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.22, 0.42));
    ink::fill1(tone(t0));
    if r > 0.05 && !ghost_on() {
        ink::circle(0.0, 0.0, 0.35 * r);
        ink::fill(WHITE, 0.75, false);
    }
    ink::circle(0.0, 0.0, 0.16 * r);
    ink::fill1(o.eye.unwrap_or_else(|| tone(0.05)));
}

// a closed bud on its own: teardrop, three sepals. Base at the origin, +y; s = length
pub fn bud(rng: &Rng, s: f64, o: &O) {
    ink::in_bloom(|| bud_(rng, s, o))
}
fn bud_(rng: &Rng, s: f64, o: &O) {
    ink::push();
    ink::scale1(s);
    let bend = rng.sym(0.1);
    let hw = |v: f64| 0.3 * sin(PI * v.powf(0.6)).powf(0.8);
    let (outline, _, _) = ink::blade(&|v| (bend * v * v, v), 18, &hw, &hw, None);
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.3, 0.42));
    ink::smooth(&outline, true);
    ink::fill1(tone(t0));
    ink::smooth(&outline, true);
    ink::clip();
    mottle(rng, -0.3, 0.0, 0.3, 1.0, 5, 0.1, t0 - 0.2, t0 + 0.2, 0);
    streak3(-0.08, 0.05, -0.1, 0.35, -0.05, 0.65, bend * 0.8, 0.95, 0.06, 0.005, WHITE, 0.7, false);
    ink::smooth(&outline, true);
    ink::stroke(tone(0.06), 1.4 * lod(), 0.3, true);
    ink::unclip();
    for k in -1..=1 {
        let kf = k as f64;
        let a = kf * 24.0 + rng.sym(6.0);
        ink::push();
        ink::translate(kf * 0.05, -0.02);
        ink::rotate(a);
        ink::scale(0.06, if k == 0 { 0.22 } else { 0.32 });
        let sw = |v: f64| sin(PI * v.powf(0.6));
        ink::smooth(&ink::blade(&|v| (-kf * 0.4 * v * v, v), 8, &sw, &sw, None).0, true);
        ink::pop();
    }
    ink::fill(tone(0.14), 0.9, false);
    ink::pop();
}

// a bellflower hanging from the origin (it hangs along -y); s = length
pub fn bell(rng: &Rng, s: f64, o: &O) {
    ink::in_bloom(|| bell_(rng, s, o))
}
fn bell_(rng: &Rng, s: f64, o: &O) {
    ink::push();
    ink::scale1(s);
    ink::rotate(rng.sym(8.0));
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.3, 0.4));
    // body: a flared tube with a three-lobed rim
    let side = |sg: f64| -> Vec<Pt> {
        (0..=10)
            .map(|i| {
                let v = i as f64 / 10.0;
                [sg * (0.12 + 0.22 * v.powf(1.4) + 0.08 * v.powi(6)), -0.02 - 0.82 * v]
            })
            .collect()
    };
    let (r, lft) = (side(1.0), side(-1.0));
    let mut pts: Vec<Pt> = r.clone();
    // lobes along the rim from right to left, pointed tips
    let tips = [[0.44, -0.98], [0.12, -1.0], [-0.16, -1.0], [-0.44, -0.98]];
    let sinus = [[0.27, -0.86], [-0.02, -0.9], [-0.29, -0.86]];
    pts.push([tips[0][0] + rng.sym(0.03), tips[0][1] + rng.sym(0.03)]);
    for i in 0..3 {
        pts.push([sinus[i][0], sinus[i][1] + rng.sym(0.02)]);
        pts.push([tips[i + 1][0] + rng.sym(0.03), tips[i + 1][1] + rng.sym(0.03)]);
    }
    pts.extend(lft.iter().rev());
    ink::smooth_t(&pts, true, 0.7);
    ink::fill1(tone(t0));
    ink::smooth_t(&pts, true, 0.7);
    ink::clip();
    mottle(rng, -0.5, -1.0, 0.5, 0.0, 10, 0.12, t0 - 0.2, t0 + 0.2, 10);
    for k in -2i64..=2 {
        let x = k as f64 * 0.1;
        let even = k.rem_euclid(2) == 0;
        streak3(x * 0.3, -0.05, x * 0.8, -0.35, x * 1.3, -0.65, x * 1.9 + rng.sym(0.03), -0.92, 0.02, 0.012,
            if even { WHITE } else { tone(0.08) }, if even { 0.55 } else { 0.3 }, !even);
    }
    ink::smooth_t(&pts, true, 0.7);
    ink::stroke(tone(0.06), 1.5 * lod(), 0.3, true);
    ink::unclip();
    ink::smooth_t(&pts, true, 0.7);
    ink::stroke(tone(0.08), 0.18, 0.35, false);
    // calyx on top
    for k in -1..=1 {
        ink::push();
        ink::translate(0.0, 0.02);
        ink::rotate(180.0 + k as f64 * 40.0 + rng.sym(8.0));
        ink::scale(0.08, 0.26);
        let sw = |v: f64| sin(PI * v.powf(0.7));
        ink::smooth(&ink::blade(&|v| (0.0, v), 8, &sw, &sw, None).0, true);
        ink::pop();
    }
    ink::fill1(tone(0.1));
    ink::circle(0.0, 0.02, 0.06);
    ink::fill1(tone(0.08));
    ink::pop();
}

// ---------------------------------------------------------------- foliage
// A leaf pointing +y, length 1, with a bent midrib, a darker half (the leaf
// folds along its rib) and veins lifted out in white. With depth > 0 it is
// lobed like acanthus: a narrow body plus pointed lobes leaning toward the tip,
// all filled as one shape; with depth = 0 it is entire and finely toothed.
// Opts: w (half-width of the envelope), lobes (per side), depth (0..1), bend, tone.
pub fn leaf(rng: &Rng, o: &O) {
    let w = o.w.unwrap_or_else(|| rng.range(0.20, 0.27));
    let bend = o.bend.unwrap_or_else(|| rng.sym(0.25));
    let lobes = o.lobes.unwrap_or_else(|| rng.int(3, 5));
    let depth = o.depth.unwrap_or_else(|| rng.range(0.3, 0.7));
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.26, 0.38));
    let u = lod();
    let fine = u > 0.6 && !ghost_on();
    let spine = |v: f64| (bend * v * v - 0.3 * bend * v, v);
    let tangent = |v: f64| {
        let (x0, y0) = spine((v - 0.01).max(0.0));
        let (x1, y1) = spine((v + 0.01).min(1.0));
        let (dx, dy) = (x1 - x0, y1 - y0);
        let l = (dx * dx + dy * dy).sqrt();
        (dx / l, dy / l)
    };
    let envelope = |v: f64| w * 2.3 * v.powf(0.55) * (1.0 - v).powf(0.95);
    let teeth = |v: f64| 0.006 * (1.0 - (2.0 * fmod(v * 9.0, 1.0) - 1.0).abs());
    let mut veins: Vec<[f64; 6]> = vec![]; // lobe axes, for the white veins
    // the body
    let bodyw = if depth > 0.0 { 1.0 - 0.75 * depth } else { 1.0 };
    let hb = |v: f64| envelope(v) * bodyw;
    let body_edge = |v: f64, _: f64| (if depth > 0.0 { 0.0 } else { 1.0 }) * teeth(v * 3.0);
    let (outline, _, _) = ink::blade(&spine, if u > 0.6 { 36 } else { 24 }, &hb, &hb, Some(&body_edge));
    let mut shapes = vec![outline.clone()];
    // the lobes
    if depth > 0.0 {
        for side in [-1.0, 1.0] {
            for k in 1..=lobes {
                let v = 0.1 + 0.72 * (k as f64 - if side > 0.0 { 0.25 } else { 0.75 }) / lobes as f64;
                let (x0, y0) = spine(v);
                let (tx, ty) = tangent(v);
                let reach = envelope((v + 0.08).min(0.95)) * rng.range(0.95, 1.15);
                let ang = rng.range(38.0, 58.0) * RAD;
                // lobe direction: the tangent turned outward by ang (side picks the side)
                let (c, sn) = (ang.cos(), ang.sin() * side);
                let (dx, dy) = (tx * c - ty * sn, tx * sn + ty * c);
                let l = reach / ang.sin() * 0.95;
                let lb = rng.sym(0.25) - side * 0.15;
                let ls = |t: f64| (x0 + dx * l * t - dy * lb * l * t * t, y0 + dy * l * t + dx * lb * l * t * t);
                let lw = |t: f64| l * rng.range(0.26, 0.3) * sin(PI * t.powf(0.62)).powf(0.75);
                let le = |t: f64, _: f64| if t > 0.2 { teeth(t * 1.4 + k as f64) * l * 4.0 } else { 0.0 };
                let (lobe, _, _) = ink::blade(&ls, 11, &lw, &lw, Some(&le));
                shapes.push(lobe);
                let (mx, my) = ls(0.45);
                let (ex, ey) = ls(0.92);
                veins.push([x0, y0, mx, my, ex, ey]);
            }
        }
    }
    let trace = || {
        for sh in &shapes {
            ink::smooth_t(sh, true, 0.8);
        }
    };
    trace();
    ink::fill1(tone(t0));
    trace();
    ink::clip();
    // the darker half
    let sd = sgn(rng.chance(0.5));
    let mut hp: Vec<Pt> = (0..=30)
        .map(|i| {
            let (x, y) = spine(i as f64 / 30.0);
            [x, y]
        })
        .collect();
    hp.push([sd * 2.0, 1.0]);
    hp.push([sd * 2.0, 0.0]);
    ink::poly(&hp, true);
    ink::fill(tone(0.12), 0.28, true);
    mottle(rng, -w * 2.5, 0.0, w * 2.5, 1.0, if fine { 13 } else { 6 }, 0.11, t0 - 0.2, t0 + 0.22, if fine { 12 } else { 0 });
    trace();
    ink::stroke(tone(0.06), 1.3 * u, 0.22, true);
    // midrib and veins in white
    streak(&spine, 10, 0.03, 0.003, WHITE, 0.78, false);
    for vn in &veins {
        let [x0, y0, mx, my, ex, ey] = *vn;
        streak3(x0, y0, mx, my, mx + (ex - mx) * 0.5, my + (ey - my) * 0.5, ex, ey, 0.017, 0.002, WHITE, rng.range(0.5, 0.75), false);
    }
    if depth == 0.0 || veins.is_empty() {
        for k in 1..=9 {
            let v = (k as f64 - 0.3) / 10.0 * 0.9;
            let sg = if k % 2 == 0 { 1.0 } else { -1.0 };
            let (x0, y0) = spine(v);
            let (tx, ty) = tangent(v);
            let (nx, ny) = (-ty * sg, tx * sg);
            let l = hb(v) * rng.range(0.75, 0.95);
            streak3(x0, y0, x0 + nx * l * 0.4, y0 + ny * l * 0.4, x0 + nx * l * 0.75 + tx * l * 0.3, y0 + ny * l * 0.75 + ty * l * 0.3,
                x0 + nx * l * 0.95 + tx * l * 0.6, y0 + ny * l * 0.95 + ty * l * 0.6, 0.014, 0.002, WHITE, rng.range(0.45, 0.7), false);
        }
    }
    ink::unclip();
    if depth == 0.0 {
        ink::smooth_t(&outline, true, 0.8);
        ink::stroke(tone(0.05), 0.18, if ghost_on() { 0.15 } else { 0.4 }, false);
    }
}

// A small entire leaf, cheap: for stem nodes and sprays. +y, length 1.
pub fn leaflet(rng: &Rng, o: &O) {
    let w = o.w.unwrap_or_else(|| rng.range(0.16, 0.24));
    let bend = o.bend.unwrap_or_else(|| rng.sym(0.2));
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.2, 0.34));
    let spine = |v: f64| (bend * v * v, v);
    let half = |v: f64| w * 2.2 * v.powf(0.5) * (1.0 - v).powf(0.9);
    let (outline, c, nrm) = ink::blade(&spine, 18, &half, &half, None);
    ink::smooth(&outline, true);
    ink::fill1(tone(t0));
    if lod() > 0.25 && !ghost_on() {
        ink::smooth(&outline, true);
        ink::clip();
        let sd = sgn(rng.chance(0.5));
        let mut hp: Vec<Pt> = c.clone();
        for i in (0..c.len()).rev() {
            hp.push([c[i][0] + sd * nrm[i][0], c[i][1] + sd * nrm[i][1]]);
        }
        ink::poly(&hp, true);
        ink::fill(tone(0.08), 0.3, true);
        streak(&spine, 10, 0.03, 0.003, WHITE, 0.65, false);
        ink::unclip();
    }
}

// ---------------------------------------------------------------- stems and sprays
// a tapered stem along points (local cm), full widths in cm at the base and the
// tip; with `tip` (cm) its last stretch narrows to a point, as at a growing tip
pub fn stem(pts: &[Pt], w0: f64, w1: f64, c: Option<Color>, op: f64, tip: Option<f64>) {
    let wbp = (w0 + w1) / 2.0 * ink::unit() * CM;
    let c = c.unwrap_or_else(|| tone(0.07));
    if wbp < 0.7 {
        ink::smooth(pts, false);
        ink::stroke(c, wbp, op, false);
        return;
    }
    let out = match tip {
        Some(tip) => {
            let (_, l) = ink::arclen(pts);
            let f = (tip / l.max(1e-9)).min(1.0);
            ink::ribbon(pts, &|t| lerp(w0, w1, t) * if t > 1.0 - f { lerp(1.0, 0.15, (t - 1.0 + f) / f) } else { 1.0 })
        }
        None => ink::ribbon(pts, &|t| lerp(w0, w1, t)),
    };
    ink::smooth(&out, true);
    ink::fill(c, op, false);
}
pub fn stem1(pts: &[Pt], w0: f64, w1: f64, c: Color) {
    stem(pts, w0, w1, Some(c), 1.0, None)
}

// The path of a branch from a stem point (sx,sy) whose stem runs along (tx,ty)
// to a motif at (bx,by) whose axis is (ax,ay): it leaves along the stem (or
// against it, if the target lies behind), turned 28..48 degrees toward the
// target, and arrives along the motif's axis. Returns sampled points.
#[allow(clippy::too_many_arguments)]
pub fn branch_path(rng: &Rng, sx: f64, sy: f64, tx: f64, ty: f64, bx: f64, by: f64, ax: f64, ay: f64) -> Vec<Pt> {
    let (dx, dy) = (bx - sx, by - sy);
    let d = (dx * dx + dy * dy).sqrt();
    let (tx, ty) = if tx * dx + ty * dy < -0.3 * d { (-tx, -ty) } else { (tx, ty) };
    let turn = sgn(tx * dy - ty * dx > 0.0) * rng.range(28.0, 48.0) * RAD;
    let (ox, oy) = (tx * turn.cos() - ty * turn.sin(), tx * turn.sin() + ty * turn.cos());
    ink::sample(
        &ink::bezier3(sx, sy, sx + ox * d * 0.45, sy + oy * d * 0.45, bx - ax * d * 0.4, by - ay * d * 0.4, bx, by),
        6usize.max((d / 0.15).floor() as usize),
    )
}

// integrate a path from a heading function: start (x,y), heading h(s) in
// degrees at arclength s, total length L, n steps
pub fn walk(mut x: f64, mut y: f64, h: &dyn Fn(f64) -> f64, l: f64, n: usize) -> Vec<Pt> {
    let ds = l / n as f64;
    let mut pts = vec![[x, y]];
    for i in 1..=n {
        let a = h((i as f64 - 0.5) * ds) * RAD;
        x += ds * a.cos();
        y += ds * a.sin();
        pts.push([x, y]);
    }
    pts
}

// A forget-me-not cyme: a stem of length L (cm) that curls at its end like a
// scorpion's tail, florets crowding along it and shrinking into buds; often a
// second, shorter curl forks off. Starts at the origin heading +y; side = +1
// curls left, -1 right.
#[allow(clippy::too_many_arguments)]
fn curl(rng: &Rng, x: f64, y: f64, h0: f64, l: f64, side: f64, t0: f64, wbase: f64) -> Vec<Pt> {
    let turn = rng.range(170.0, 270.0);
    const N: usize = 36;
    let pts = walk(x, y, &|s| h0 + side * turn * (s / l).powf(2.3), l, N);
    stem1(&pts, wbase, 0.006, tone(0.1));
    let nrm = ink::normals(&pts);
    let n = rng.int(11, 17);
    for k in 1..=n {
        let f = 0.18 + 0.82 * (k as f64 / n as f64).powf(0.75);
        let i = ((f * N as f64 + 1.5).floor() as usize).clamp(2, pts.len()) - 1;
        let sg = if k % 2 == 0 { 1.0 } else { -1.0 };
        let (px, py) = (pts[i][0], pts[i][1]);
        let stalk = lerp(0.1, 0.02, f) * rng.range(0.7, 1.2);
        let (qx, qy) = (px + nrm[i][0] * sg * stalk, py + nrm[i][1] * sg * stalk);
        if stalk > 0.03 {
            ink::moveto(px, py);
            ink::lineto(qx, qy);
            ink::stroke(tone(0.12), 0.25, 0.9, false);
        }
        let r = lerp(0.115, 0.03, f) * rng.range(0.85, 1.15);
        ink::push();
        ink::translate(qx, qy);
        if f > 0.8 {
            ink::circle(0.0, 0.0, r * 0.6);
            ink::fill1(tone(t0 - 0.06));
        } else {
            floret(rng, r, &O { star: Some(false), tone: Some(t0 + rng.sym(0.08)), ..d() });
        }
        ink::pop();
    }
    pts
}

pub fn cyme(rng: &Rng, l: f64, o: &O) {
    let side = o.side.unwrap_or_else(|| sgn(rng.chance(0.5)));
    let t0 = rng.range(0.22, 0.36);
    let pts = curl(rng, 0.0, 0.0, 90.0 + rng.sym(10.0), l, side, t0, 0.026);
    if o.fork != Some(false) && rng.chance(if o.fork == Some(true) { 1.0 } else { 0.6 }) {
        let i = (pts.len() as f64 * rng.range(0.25, 0.4)).floor() as usize; // 1-based
        let (a, b) = (pts[i], pts[i - 1]);
        let h = (a[1] - b[1]).atan2(a[0] - b[0]) / RAD;
        let h1 = h - side * rng.range(30.0, 50.0);
        let l1 = l * rng.range(0.45, 0.65);
        curl(rng, b[0], b[1], h1, l1, -side, t0 + 0.04, 0.018);
    }
}

// A wiry branching sprig (baby's breath): forks two or three ways `depth`
// times, small leaves at the lower forks, florets and buds at the tips.
// Starts at the origin heading +y, length L (cm).
#[allow(clippy::too_many_arguments)]
fn sprig_branch(rng: &Rng, t0: f64, depth: i64, x: f64, y: f64, a: f64, len: f64, d: i64, w: f64) {
    let bend = rng.sym(20.0);
    let pts = walk(x, y, &|s| a + bend * s / len, len, 6);
    stem1(&pts, w, w * 0.7, tone(0.1));
    let (ex, ey) = (pts[pts.len() - 1][0], pts[pts.len() - 1][1]);
    let ea = a + bend;
    if d == 0 {
        ink::push();
        ink::translate(ex, ey);
        if rng.chance(0.7) {
            let r = rng.range(0.045, 0.075);
            floret(rng, r, &O { tone: Some(t0 + rng.sym(0.08)), ..O::default() });
        } else {
            ink::circle(0.0, 0.0, rng.range(0.018, 0.028));
            ink::fill1(tone(t0 - 0.05));
        }
        ink::pop();
        return;
    }
    if d >= depth - 1 && rng.chance(0.7) {
        let sg = sgn(rng.chance(0.5));
        ink::push();
        ink::translate(ex, ey);
        ink::rotate(ea - 90.0 + sg * rng.range(40.0, 70.0));
        ink::scale1(len * rng.range(0.5, 0.8));
        leaflet(rng, &O { w: Some(0.14), ..O::default() });
        ink::pop();
    }
    let kids = if rng.chance(0.25) { 3 } else { 2 };
    for k in 1..=kids {
        let da = if kids == 2 { (if k == 1 { -1.0 } else { 1.0 }) * rng.range(16.0, 34.0) } else { (k - 2) as f64 * rng.range(22.0, 36.0) };
        sprig_branch(rng, t0, depth, ex, ey, ea + da, len * rng.range(0.58, 0.74), d - 1, w * 0.72);
    }
}
pub fn sprig(rng: &Rng, l: f64, o: &O) {
    let depth = o.depth.map_or(4, |d| d as i64);
    let t0 = rng.range(0.22, 0.4);
    let a = 90.0 + rng.sym(8.0);
    sprig_branch(rng, t0, depth, 0.0, 0.0, a, l * 0.36, depth, 0.024);
}

// Queen Anne's lace: rays fanning upward, each ending in a small umbel of starry
// florets; bracts at the base. R = reach (cm); base at the origin.
pub fn umbel(rng: &Rng, r: f64, _o: &O) {
    let nr = rng.int(9, 13);
    for k in 1..=nr {
        let a = 90.0 + (k as f64 - (nr + 1) as f64 / 2.0) * (110.0 / nr as f64) + rng.sym(4.0);
        let l = r * rng.range(0.78, 1.0) * (1.0 - 0.15 * (a - 90.0).abs() / 55.0);
        let (ex, ey) = (l * (a * RAD).cos(), l * (a * RAD).sin());
        ink::smooth(&ink::sample(&ink::bezier3(0.0, 0.0, ex * 0.15, ey * 0.5, ex * 0.6, ey * 0.95, ex, ey), 6), false);
        ink::stroke(tone(0.12), 0.25, 0.9, false);
        let ns = rng.int(5, 8);
        for j in 1..=ns {
            let b = a + (j as f64 - (ns + 1) as f64 / 2.0) * 16.0 + rng.sym(5.0);
            let l = r * rng.range(0.12, 0.2);
            let (fx, fy) = (ex + l * (b * RAD).cos(), ey + l * (b * RAD).sin());
            ink::moveto(ex, ey);
            ink::lineto(fx, fy);
            ink::stroke(tone(0.14), 0.18, 0.85, false);
            ink::push();
            ink::translate(fx, fy);
            let fr = r * rng.range(0.035, 0.05);
            floret(rng, fr, &O { star: Some(true), tone: Some(rng.range(0.25, 0.42)), ..d() });
            ink::pop();
        }
    }
    for k in -1..=1 {
        ink::push();
        ink::rotate(k as f64 * 40.0 + rng.sym(10.0) + 180.0);
        ink::scale1(r * rng.range(0.15, 0.22));
        leaflet(rng, &O { w: Some(0.06), bend: Some(k as f64 * 0.2), ..d() });
        ink::pop();
    }
}

// A flower gone to seed, a dandelion clock: hairline rays from a small dark
// head, each ending in a tuft, a few gaps where seeds have gone, and a few
// seeds drifting away up and to one side. Center at the origin; r = radius (cm).
fn tuft(x: f64, y: f64, a: f64, l: f64) {
    for j in -2..=2 {
        let b = a + j as f64 * 0.32;
        ink::moveto(x, y);
        ink::lineto(x + l * b.cos(), y + l * b.sin());
    }
}
pub fn seedhead(rng: &Rng, r: f64, o: &O) {
    let fine = lod() * r > 0.2 && !ghost_on();
    let n = if fine { rng.int(26, 34) } else { 16 };
    let gone = rng.range(0.0, 2.0 * PI);
    for k in 1..=n {
        let a = 2.0 * PI * (k as f64 + rng.sym(0.3)) / n as f64;
        if !((fmod(a - gone + PI, 2.0 * PI) - PI).abs() < 0.5 && rng.chance(0.7)) {
            let l = r * rng.range(0.72, 0.86);
            let (c, s) = (a.cos(), a.sin());
            ink::moveto(0.1 * r * c, 0.1 * r * s);
            ink::lineto(l * c, l * s);
            if fine {
                tuft(l * c, l * s, a, r * rng.range(0.14, 0.2));
            }
        }
    }
    ink::stroke(tone(0.16), 0.22, 0.85, false);
    ink::circle(0.0, 0.0, 0.13 * r);
    ink::fill1(tone(0.12));
    if fine {
        for _ in 0..10 {
            speck(rng.sym(0.08 * r), rng.sym(0.08 * r), 0.03 * r, rng.range(0.0, PI));
        }
        ink::fill(tone(0.04), 0.8, false);
    }
    // drifting seeds: an achene on a beak, the tuft above it
    let side = o.side.unwrap_or_else(|| sgn(rng.chance(0.5)));
    for _ in 0..o.drift.unwrap_or_else(|| rng.int(2, 4)) {
        let dd = r * rng.range(1.3, 2.4);
        let a = (90.0 - side * rng.range(15.0, 60.0)) * RAD;
        let (x, y) = (dd * a.cos(), dd * a.sin());
        let tilt = (90.0 + rng.sym(35.0)) * RAD;
        let l = r * 0.32;
        let (tx, ty) = (x + l * tilt.cos(), y + l * tilt.sin());
        ink::moveto(x, y);
        ink::lineto(tx, ty);
        tuft(tx, ty, tilt, r * 0.17);
        ink::stroke(tone(0.18), 0.2, 0.8, false);
        ink::push();
        ink::translate(x, y);
        ink::rotate(tilt / RAD - 90.0);
        let hw = |v: f64| 0.025 * r * sin(PI * v);
        ink::smooth(&ink::blade(&|v| (0.0, -0.12 * r * v), 6, &hw, &hw, None).0, true);
        ink::fill1(tone(0.1));
        ink::pop();
    }
}

// a tapered logarithmic spiral, s = size (cm); starts at the origin, outer end
pub fn tendril(rng: &Rng, s: f64, _o: &O) {
    let turns = rng.range(1.6, 2.3);
    let pts: Vec<Pt> = (0..=50)
        .map(|i| {
            let t = i as f64 / 50.0;
            let a = t * turns * 2.0 * PI;
            let r = s * (1.0 - 0.92 * t).powf(1.6);
            [r * a.cos() - s, r * a.sin()]
        })
        .collect();
    stem1(&pts, 0.022, 0.004, tone(0.12));
}

// ---------------------------------------------------------------- accents (copper hairlines)
// a four-point star with a hairline cross; r = radius (cm)
pub fn sparkle(_rng: &Rng, r: f64, o: &O) {
    let c = o.color.unwrap_or_else(|| ink::accent(0.0));
    let k = 0.12;
    ink::poly(&[[0.0, r], [k * r, k * r], [r * 0.75, 0.0], [k * r, -k * r], [0.0, -r], [-k * r, -k * r], [-r * 0.75, 0.0], [-k * r, k * r]], true);
    ink::fill1(c);
    if o.rays != Some(false) {
        ink::moveto(0.0, r * 1.15);
        ink::lineto(0.0, r * 1.7);
        ink::moveto(0.0, -r * 1.15);
        ink::lineto(0.0, -r * 1.7);
        ink::stroke(c, 0.2, 0.8, false);
        ink::circle(0.0, r * 1.9, r * 0.07);
        ink::circle(0.0, -r * 1.9, r * 0.07);
        ink::fill(c, 0.8, false);
    }
}

// a tiny hairline quatrefoil with a dot; r = radius (cm)
pub fn rosette(_rng: &Rng, r: f64, o: &O) {
    let c = o.color.unwrap_or_else(|| ink::accent(0.0));
    for k in 0..4 {
        let a = k as f64 * PI / 2.0 + PI / 4.0;
        ink::circle(r * 0.5 * a.cos(), r * 0.5 * a.sin(), r * 0.42);
    }
    ink::stroke(c, 0.22, 0.85, false);
    ink::circle(0.0, 0.0, r * 0.14);
    ink::fill1(c);
    ink::moveto(0.0, r * 1.05);
    ink::lineto(0.0, r * 1.5);
    ink::moveto(0.0, -r * 1.05);
    ink::lineto(0.0, -r * 1.5);
    ink::stroke(c, 0.2, 0.7, false);
}

// the header fleuron: a printer's ivy leaf (hedera) in copper, tip down-left,
// its stem curling once and running out to the right, where the head rule
// continues. Origin at the leaf's notch; returns the end of the stem in page cm.
pub fn hedera(_rng: &Rng, s: f64, o: &O) -> (f64, f64) {
    let c = o.color.unwrap_or_else(|| ink::accent(0.0));
    ink::push();
    ink::scale1(s);
    let pts: Vec<Pt> = (0..48)
        .map(|i| {
            let t = i as f64 / 48.0 * 2.0 * PI;
            let x = 16.0 * t.sin().powi(3);
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            [x / 17.0, (y - 5.0) / 17.0] // notch at the origin, tip at (0,-1.3)
        })
        .collect();
    ink::push();
    ink::rotate(-38.0);
    ink::smooth(&pts, true);
    ink::fill1(c);
    // veins scratched out of the copper
    ink::moveto(0.0, -0.05);
    ink::curveto(0.02, -0.4, -0.02, -0.8, 0.0, -1.12);
    ink::moveto(0.0, -0.35);
    ink::curveto(0.15, -0.38, 0.3, -0.32, 0.42, -0.22);
    ink::moveto(0.0, -0.35);
    ink::curveto(-0.15, -0.38, -0.3, -0.32, -0.42, -0.22);
    ink::stroke1(ink::PAPER, 0.35);
    ink::pop();
    // stem: up out of the notch, one curl, then away to the right
    let st = [[0.0, 0.0], [0.12, 0.28], [0.38, 0.42], [0.6, 0.3], [0.52, 0.1], [0.36, 0.2], [0.48, 0.36], [0.85, 0.36], [1.4, 0.3], [2.0, 0.28]];
    stem1(&st, 0.05, 0.012, c);
    ink::circle(-0.25, 0.42, 0.04);
    ink::circle(0.95, 0.6, 0.035);
    ink::circle(0.7, -0.05, 0.03);
    ink::fill(c, 0.85, false);
    let r = ink::to_page(2.0, 0.28);
    ink::pop();
    r
}

// ---------------------------------------------------------------- visitors
fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64, n: usize) -> Vec<Pt> {
    (0..n)
        .map(|i| {
            let a = 2.0 * PI * i as f64 / n as f64;
            [cx + rx * a.cos(), cy + ry * a.sin()]
        })
        .collect()
}

// one wing (or any flat shape) painted like a petal: wash, blotches, a dark
// pooled border, white veins fanning from the root (rx, ry), pale spots at the rim
#[allow(clippy::too_many_arguments)]
fn wing(rng: &Rng, pts: &[Pt], t0: f64, rx: f64, ry: f64, spots: bool, fine: bool) {
    ink::smooth(pts, true);
    ink::fill1(tone(t0));
    ink::smooth(pts, true);
    ink::clip();
    mottle(rng, -1.2, -1.2, 1.2, 1.2, if fine { 9 } else { 4 }, 0.14, t0 - 0.2, t0 + 0.2, if fine { 8 } else { 0 });
    ink::smooth(pts, true);
    ink::stroke(tone(0.05), 0.26 * lod() * CM, if ghost_on() { 0.25 } else { 0.55 }, true);
    for (i, p) in pts.iter().enumerate() {
        let f = rng.range(0.7, 0.88);
        let (ex, ey) = (rx + (p[0] - rx) * f, ry + (p[1] - ry) * f);
        streak3(rx, ry, lerp(rx, ex, 0.3), lerp(ry, ey, 0.4), lerp(rx, ex, 0.7), lerp(ry, ey, 0.75), ex, ey, 0.035, 0.004, WHITE, rng.range(0.35, 0.6), false);
        if spots && fine && i > 0 && i < pts.len() - 1 {
            let (cx, cy) = (rx + (p[0] - rx) * 0.9, ry + (p[1] - ry) * 0.9);
            ink::circle(cx, cy, rng.range(0.025, 0.04));
            ink::fill(WHITE, 0.8, false);
        }
    }
    ink::unclip();
}

// A butterfly seen from above, wings spread, head toward +y, the body's center
// at the origin; s = wingspan (cm). o.open in (0,1] narrows the wings: a wingbeat.
pub fn butterfly(rng: &Rng, s: f64, o: &O) {
    let open = o.open.unwrap_or(1.0);
    ink::push();
    ink::scale1(s / 2.0); // one unit = half the span
    let fine = lod() > 0.3 && !ghost_on();
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.28, 0.4));
    let mut fw: Vec<Pt> = vec![[0.05, 0.1], [0.3, 0.45], [0.65, 0.68], [0.92, 0.66], [1.0, 0.5], [0.88, 0.25], [0.55, 0.05], [0.15, -0.02]];
    let mut hw: Vec<Pt> = vec![[0.06, -0.02], [0.45, -0.02], [0.72, -0.22], [0.7, -0.5], [0.5, -0.72], [0.25, -0.7], [0.08, -0.35]];
    for w in [&mut fw, &mut hw] {
        for p in w.iter_mut() {
            let (a, b) = (p[0] + rng.sym(0.03), p[1] + rng.sym(0.03));
            *p = [a, b];
        }
    }
    let eye = rng.chance(0.7);
    for sx in [-1.0, 1.0] {
        let side: Vec<Pt> = hw.iter().map(|p| [sx * p[0] * open, p[1]]).collect();
        wing(rng, &side, t0 + 0.06, sx * 0.04 * open, -0.04, true, fine);
        if eye && fine {
            // an eyespot with a copper pupil
            let (ex, ey) = (sx * 0.43 * open, -0.42);
            ink::smooth(&ellipse(ex, ey, 0.11 * open, 0.11, 10), true);
            ink::fill(tone(0.08), 0.85, false);
            ink::smooth(&ellipse(ex, ey, 0.07 * open, 0.07, 10), true);
            ink::fill(WHITE, 0.85, false);
            ink::smooth(&ellipse(ex, ey, 0.035 * open, 0.035, 8), true);
            ink::fill1(ink::accent(0.0));
        }
    }
    for sx in [-1.0, 1.0] {
        let w: Vec<Pt> = fw.iter().map(|p| [sx * p[0] * open, p[1]]).collect();
        wing(rng, &w, t0, sx * 0.04 * open, 0.04, true, fine);
    }
    // body: abdomen, furry thorax, head, clubbed antennae
    let hb = |v: f64| 0.05 * sin(PI * v.powf(0.7));
    ink::smooth(&ink::blade(&|v| (0.0, -0.62 + 0.62 * v), 10, &hb, &hb, None).0, true);
    ink::fill1(tone(0.06));
    ink::smooth(&ellipse(0.0, 0.08, 0.075, 0.13, 10), true);
    ink::fill1(tone(0.04));
    ink::circle(0.0, 0.27, 0.06);
    ink::fill1(tone(0.04));
    if fine {
        for sx in [-1.0, 1.0] {
            let (ex, ey) = (sx * rng.range(0.22, 0.3), rng.range(0.78, 0.9));
            ink::moveto(sx * 0.02, 0.31);
            ink::curveto(sx * 0.05, 0.5, ex * 0.6, ey * 0.9, ex, ey);
            ink::stroke1(tone(0.08), 0.35);
            ink::circle(ex, ey, 0.025);
            ink::fill1(tone(0.06));
        }
        streak3(-0.015, -0.55, -0.02, -0.35, -0.02, -0.1, -0.01, 0.0, 0.015, 0.004, WHITE, 0.45, false);
    }
    ink::pop();
}

// A bee from above, head toward +y, center at the origin; s = body length (cm).
// o.trail: a dotted flight path looping away behind it.
pub fn bee(rng: &Rng, s: f64, o: &O) {
    ink::push();
    ink::scale1(s);
    let fine = lod() > 0.25 && !ghost_on();
    if o.trail != Some(false) && !ghost_on() {
        let (r, c, turns) = (0.32, rng.range(0.16, 0.22), rng.range(1.2, 1.6));
        let side = sgn(rng.chance(0.5));
        let pts: Vec<Pt> = (0..=60)
            .map(|i| {
                let th = i as f64 / 60.0 * turns * 2.0 * PI;
                [side * r * th.sin(), -0.62 - c * th + r * (th.cos() - 1.0) * 0.6]
            })
            .collect();
        ink::smooth(&pts, false);
        ink::stroke_d(tone(0.18), 0.9, 0.7, false, Some("[0 2.6] 0"));
    }
    // abdomen with dark bands, a sting
    let ab = ellipse(0.0, -0.2, 0.19, 0.3, 16);
    ink::smooth(&ab, true);
    ink::fill1(tone(0.38));
    ink::smooth(&ab, true);
    ink::clip();
    for k in 0..3 {
        let y = -0.06 - 0.14 * k as f64;
        ink::poly(&[[-0.3, y + 0.035], [0.0, y + 0.06], [0.3, y + 0.035], [0.3, y - 0.035], [0.0, y - 0.01], [-0.3, y - 0.035]], true);
    }
    ink::fill(tone(0.05), 0.9, false);
    if fine {
        mottle(rng, -0.2, -0.5, 0.2, 0.1, 4, 0.06, 0.2, 0.55, 4);
    }
    ink::smooth(&ab, true);
    ink::stroke(tone(0.05), 0.12 * lod() * CM, 0.4, true);
    streak3(-0.1, -0.38, -0.13, -0.25, -0.12, -0.12, -0.08, 0.0, 0.04, 0.008, WHITE, 0.55, false);
    ink::unclip();
    ink::poly(&[[-0.025, -0.48], [0.0, -0.58], [0.025, -0.48]], true);
    ink::fill1(tone(0.05));
    // legs, then a furry thorax and the head
    if fine {
        for sx in [-1.0, 1.0] {
            for k in 0..3 {
                let y0 = 0.17 - 0.06 * k as f64;
                let ex = sx * rng.range(0.26, 0.32);
                let ey = y0 - 0.12 + 0.08 * (1 - k) as f64 + rng.sym(0.03);
                ink::moveto(sx * 0.08, y0);
                ink::curveto(sx * 0.18, y0 + 0.04, ex * 0.9, ey + 0.05, ex, ey);
            }
        }
        ink::stroke1(tone(0.08), 0.35);
    }
    ink::smooth(&ink::blob(rng, 0.0, 0.15, 0.14, 0.13, 0.18, 12), true);
    ink::fill1(tone(0.06));
    if fine {
        mottle(rng, -0.14, 0.03, 0.14, 0.28, 3, 0.05, 0.02, 0.3, 6);
    }
    ink::smooth(&ellipse(0.0, 0.33, 0.09, 0.075, 10), true);
    ink::fill1(tone(0.04));
    for sx in [-1.0, 1.0] {
        ink::moveto(sx * 0.03, 0.38);
        ink::lineto(sx * 0.07, 0.48);
        ink::lineto(sx * 0.14, 0.53);
    }
    ink::stroke1(tone(0.06), 0.3);
    // wings over the body: clear, veined, a little blue
    for sx in [-1.0, 1.0] {
        for (k, wl) in [[0.5, 0.16, 32.0], [0.36, 0.12, 52.0]].iter().enumerate() {
            ink::push();
            ink::translate(sx * 0.07, 0.17 - 0.04 * (k + 1) as f64);
            ink::rotate(if sx > 0.0 { -wl[2] } else { 180.0 + wl[2] });
            let w = ellipse(wl[0] / 2.0, 0.0, wl[0] / 2.0, wl[1] / 2.0, 14);
            ink::smooth(&w, true);
            ink::fill(tone(0.85), 0.55, false);
            ink::smooth(&w, true);
            ink::stroke(tone(0.15), 0.3, 0.75, false);
            if fine {
                ink::moveto(0.02, 0.0);
                ink::curveto(wl[0] * 0.3, 0.02, wl[0] * 0.6, 0.01, wl[0] * 0.85, 0.0);
                ink::moveto(wl[0] * 0.3, 0.01);
                ink::lineto(wl[0] * 0.55, wl[1] * 0.4);
                ink::stroke(tone(0.2), 0.2, 0.6, false);
            }
            ink::pop();
        }
    }
    ink::pop();
}

// A snail crawling up along +y, its foot on the y axis and its shell on the +x
// side, origin at the middle of the foot; s = shell diameter (cm).
pub fn snail(rng: &Rng, s: f64, o: &O) {
    ink::push();
    ink::scale1(s);
    let fine = lod() > 0.25 && !ghost_on();
    // the body: a long soft foot, the head raised at the front
    let mut body: Vec<Pt> = vec![[0.0, -0.78], [-0.01, -0.3], [0.0, 0.3], [0.04, 0.58], [0.12, 0.7], [0.22, 0.66],
        [0.24, 0.5], [0.2, 0.32], [0.17, 0.0], [0.12, -0.45], [0.05, -0.7]];
    for p in body.iter_mut() {
        p[0] *= 1.0 + rng.sym(0.08);
    }
    let tb = o.tone.unwrap_or_else(|| rng.range(0.5, 0.6));
    ink::smooth(&body, true);
    ink::fill1(tone(tb));
    ink::smooth(&body, true);
    ink::clip();
    if fine {
        mottle(rng, -0.1, -0.8, 0.3, 0.8, 5, 0.1, tb - 0.25, tb + 0.15, 10);
    }
    ink::moveto(-0.05, -0.8);
    ink::lineto(-0.05, 0.7);
    ink::lineto(0.04, 0.7);
    ink::lineto(0.04, -0.8);
    ink::close();
    ink::fill(tone(0.15), 0.35, true);
    ink::smooth(&body, true);
    ink::stroke(tone(0.08), 0.1 * lod() * CM, 0.4, true);
    ink::unclip();
    // feelers: two long ones with eyes, two short ones
    for (k, f) in [[0.17, 0.66, 0.36, 1.02], [0.1, 0.7, 0.18, 1.04], [0.05, 0.68, 0.0, 0.84]].iter().enumerate() {
        ink::moveto(f[0], f[1]);
        ink::curveto(f[0] + 0.02, f[1] + 0.12, f[2] - 0.04, f[3] - 0.1, f[2], f[3]);
        ink::stroke1(tone(0.12), if k < 2 { 0.45 } else { 0.35 });
        if k < 2 {
            ink::circle(f[2], f[3], 0.028);
            ink::fill1(tone(0.05));
        }
    }
    // the shell: a disc with a white spiral lifted out of it and a dark one beside it
    let (cx, cy, r) = (0.36, 0.0, 0.42);
    let t0 = rng.range(0.22, 0.32);
    let disc = ink::blob(rng, cx, cy, r, r * 0.96, 0.03, 16);
    ink::smooth(&disc, true);
    ink::fill1(tone(t0));
    ink::smooth(&disc, true);
    ink::clip();
    mottle(rng, cx - r, cy - r, cx + r, cy + r, if fine { 7 } else { 3 }, 0.1, t0 - 0.18, t0 + 0.22, if fine { 10 } else { 0 });
    let (turns, ph) = (rng.range(2.3, 2.7), rng.range(0.0, 2.0 * PI));
    let mut sp = vec![];
    let mut dk = vec![];
    for i in 0..=50 {
        let t = i as f64 / 50.0;
        let a = ph + t * turns * 2.0 * PI;
        let rr = r * 0.95 * t.powf(1.15);
        sp.push([cx + rr * a.cos(), cy + rr * a.sin()]);
        let r2 = r * 0.95 * (t - 0.12).max(0.0).powf(1.15);
        dk.push([cx + r2 * a.cos(), cy + r2 * a.sin()]);
    }
    ink::smooth(&dk, false);
    ink::stroke(tone(0.05), 0.07 * lod() * CM, 0.6, true);
    ink::smooth(&sp, false);
    ink::stroke(WHITE, 0.05 * lod() * CM, 0.75, false);
    ink::smooth(&disc, true);
    ink::stroke(tone(0.05), 0.12 * lod() * CM, 0.5, true);
    ink::unclip();
    ink::pop();
}

// A small songbird perched, seen from the side, facing +x (o.facing = -1: -x);
// its feet at the origin, s = length from tail to beak (cm).
pub fn bird(rng: &Rng, s: f64, o: &O) {
    ink::push();
    ink::scale(o.facing.unwrap_or(1.0) * s, s);
    let fine = lod() > 0.3 && !ghost_on();
    let t0 = o.tone.unwrap_or_else(|| rng.range(0.3, 0.38));
    // the tail: a few long feathers fanning down and back
    let hw = |v: f64| 0.055 * sin(PI * v.powf(0.4)).powf(0.6);
    for k in 1..=4 {
        let l = rng.range(0.42, 0.5);
        ink::push();
        ink::translate(-0.26, 0.28);
        ink::rotate(rng.range(-150.0, -140.0) + 7.0 * k as f64 - 90.0);
        let (f, _, _) = ink::blade(&|v| (0.0, v * l), 10, &hw, &hw, None);
        ink::smooth(&f, true);
        ink::fill1(tone(0.12 + 0.05 * k as f64));
        ink::smooth(&f, true);
        ink::stroke(tone(0.05), 0.3, 0.5, false);
        ink::pop();
    }
    // body and head in one outline
    let mut body: Vec<Pt> = vec![[0.37, 0.56], [0.3, 0.665], [0.15, 0.64], [-0.03, 0.54], [-0.22, 0.4], [-0.31, 0.3],
        [-0.27, 0.22], [-0.08, 0.12], [0.12, 0.12], [0.27, 0.27], [0.35, 0.42]];
    for p in body.iter_mut() {
        let (a, b) = (p[0] + rng.sym(0.012), p[1] + rng.sym(0.012));
        *p = [a, b];
    }
    ink::smooth(&body, true);
    ink::fill1(tone(t0));
    ink::smooth(&body, true);
    ink::clip();
    // pale breast with darker specks, a darker cap
    for k in 1..=4 {
        let k = k as f64;
        ink::smooth(&ink::blob(rng, 0.17, 0.27, 0.08 + 0.04 * k, 0.06 + 0.03 * k, 0.2, 9), true);
        ink::fill(WHITE, 0.2, false);
    }
    mottle(rng, -0.35, 0.1, 0.4, 0.7, if fine { 10 } else { 4 }, 0.07, t0 - 0.2, t0 + 0.2, if fine { 10 } else { 0 });
    if fine {
        for _ in 0..14 {
            let (x, y) = (rng.range(0.08, 0.3), rng.range(0.16, 0.42));
            ink::push();
            ink::translate(x, y);
            ink::rotate(rng.range(-20.0, 20.0));
            ink::smooth(&ellipse(0.0, 0.0, 0.012, 0.022, 6), true);
            ink::pop();
        }
        ink::fill(tone(0.1), 0.7, false);
    }
    ink::smooth(&ink::blob(rng, 0.24, 0.66, 0.15, 0.08, 0.1, 9), true);
    ink::fill(tone(0.08), 0.6, true);
    ink::smooth(&body, true);
    ink::stroke(tone(0.05), 0.07 * lod() * CM, 0.5, true);
    ink::unclip();
    // the folded wing: a dark blade with feather tips, white lifted along the feathers
    let wingpts = [[0.17, 0.47], [0.04, 0.52], [-0.12, 0.45], [-0.28, 0.33], [-0.44, 0.22], [-0.35, 0.23],
        [-0.38, 0.19], [-0.26, 0.21], [-0.28, 0.17], [-0.12, 0.23], [0.05, 0.31], [0.15, 0.4]];
    let tw = t0 - 0.1;
    ink::smooth_t(&wingpts, true, 0.8);
    ink::fill1(tone(tw));
    ink::smooth_t(&wingpts, true, 0.8);
    ink::clip();
    mottle(rng, -0.45, 0.15, 0.2, 0.6, if fine { 6 } else { 3 }, 0.07, tw - 0.15, tw + 0.2, 0);
    for k in 1..=5 {
        let k = k as f64;
        let y = 0.28 + 0.04 * k;
        streak3(0.12, y + 0.05, 0.0, y + 0.03, -0.15, y - 0.03, -0.3 - 0.02 * k, y - 0.1 - 0.01 * k, 0.025, 0.003, WHITE, rng.range(0.45, 0.7), false);
    }
    ink::smooth_t(&wingpts, true, 0.8);
    ink::stroke(tone(0.04), 0.06 * lod() * CM, 0.5, true);
    ink::unclip();
    // beak, eye, legs
    ink::poly(&[[0.355, 0.585], [0.47, 0.545], [0.36, 0.51]], true);
    ink::fill1(tone(0.05));
    ink::moveto(0.36, 0.548);
    ink::lineto(0.45, 0.546);
    ink::stroke(WHITE, 0.2, 0.6, false);
    ink::circle(0.285, 0.585, 0.03);
    ink::fill(WHITE, 0.95, false);
    ink::circle(0.29, 0.585, 0.018);
    ink::fill1(tone(0.03));
    ink::circle(0.284, 0.592, 0.006);
    ink::fill1(WHITE);
    for lx in [0.0, 0.08] {
        ink::moveto(lx + 0.02, 0.14);
        ink::lineto(lx, 0.0);
        ink::moveto(lx - 0.05, 0.0);
        ink::lineto(lx + 0.05, 0.005);
    }
    ink::stroke1(tone(0.06), 0.45);
    ink::pop();
}

// draw `f` as a pale ghost (the faded layer behind the garden)
pub fn ghost(level: f64, f: impl FnOnce()) {
    let old = ink::set_lift(level);
    f();
    ink::set_lift(old);
}

// by name, for \florilegiumdraw-style specimens; s is the size argument
// (petal, leaf and leaflet are drawn at unit size under ink::scale(s))
pub fn draw(name: &str, rng: &Rng, s: f64, o: &O) -> bool {
    match name {
        "petal" | "leaf" | "leaflet" => {
            ink::scale1(s);
            match name {
                "petal" => {
                    petal(rng, o);
                }
                "leaf" => leaf(rng, o),
                _ => leaflet(rng, o),
            }
        }
        "flower" => flower(rng, s, o),
        "lily" => lily(rng, s, o),
        "blossom" => blossom(rng, s, o),
        "floret" => floret(rng, s, o),
        "bud" => bud(rng, s, o),
        "bell" => bell(rng, s, o),
        "cyme" => cyme(rng, s, o),
        "sprig" => sprig(rng, s, o),
        "umbel" => umbel(rng, s, o),
        "seedhead" => seedhead(rng, s, o),
        "tendril" => tendril(rng, s, o),
        "sparkle" => sparkle(rng, s, o),
        "rosette" => rosette(rng, s, o),
        "hedera" => {
            hedera(rng, s, o);
        }
        "butterfly" => butterfly(rng, s, o),
        "bee" => bee(rng, s, o),
        "snail" => snail(rng, s, o),
        "bird" => bird(rng, s, o),
        _ => return false,
    }
    true
}
