// ink.rs -- the drawing core of florilegium: geometry in Rust, raw PDF out.
//
// A port of ink.lua. A drawing is made in a local frame (centimeters) under an
// affine transform that maps it to the page (big points, y up from the lower-left
// corner); strokes are absolute widths in bp, and anything that should scale with
// the motif is drawn as a filled ribbon instead of a stroke. Output goes into
// numbered layers of PDF path operators, the same ones ink.lua writes; lib.rs
// wraps them in a one-page PDF, which Typst embeds as it is.
//
// Color comes from a tone ramp derived from the document color `ink`: tone(0)
// is the darkest navy, tone(1) the palest mist, all at the hue of `ink`,
// computed in OKLCH so the light end stays blue instead of turning gray.
//
// The state is global, as in the Lua original: a plugin call is single-threaded
// and starts with `reset()`.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;
use std::fmt::Write;

pub const CM: f64 = 72.0 / 2.54; // bp per cm
pub const RAD: f64 = PI / 180.0;

pub type Pt = [f64; 2];
pub type Color = [f64; 3];

// Lua's floored modulo (Rust's % truncates)
pub fn fmod(a: f64, b: f64) -> f64 {
    a - (a / b).floor() * b
}
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

// ---------------------------------------------------------------- random
// mulberry32: small, fast, identical on every platform. The state is a Cell so
// that closures handed to the primitives can draw from a shared stream.
pub struct Rng {
    s: Cell<u32>,
}

impl Rng {
    pub fn new(seed: f64) -> Rng {
        Rng { s: Cell::new(seed.floor() as i64 as u32) }
    }
    pub fn u(&self) -> f64 {
        let s = self.s.get().wrapping_add(0x6D2B79F5);
        self.s.set(s);
        let mut t = s;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t = t ^ t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        (t ^ (t >> 14)) as f64 / 4294967296.0
    }
    pub fn range(&self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.u()
    }
    pub fn sym(&self, a: f64) -> f64 {
        a * (2.0 * self.u() - 1.0)
    }
    pub fn int(&self, a: i64, b: i64) -> i64 {
        a + ((b - a + 1) as f64 * self.u()).floor() as i64
    }
    pub fn chance(&self, p: f64) -> bool {
        self.u() < p
    }
    // an independent stream
    pub fn fork(&self) -> Rng {
        Rng::new((self.u() * 4294967295.0).floor())
    }
    // replay a draw (animation)
    pub fn save(&self) -> u32 {
        self.s.get()
    }
    pub fn load(&self, v: u32) {
        self.s.set(v)
    }
}

// ---------------------------------------------------------------- noise
// 1-D gradient noise with a seeded gradient table, smooth, roughly in [-1,1]
pub struct Noise {
    g: [f64; 256],
}
impl Noise {
    pub fn new(rng: &Rng) -> Noise {
        let mut g = [0.0; 256];
        for v in g.iter_mut() {
            *v = rng.sym(1.0);
        }
        Noise { g }
    }
    pub fn at(&self, x: f64) -> f64 {
        let i = x.floor() as i64;
        let f = x - i as f64;
        let a = self.g[(i & 255) as usize] * f;
        let b = self.g[((i + 1) & 255) as usize] * (f - 1.0);
        let u = f * f * (3.0 - 2.0 * f);
        2.0 * (a + (b - a) * u)
    }
}

// fractal sum of 1-D noise: each octave double frequency, half amplitude
pub struct Fbm {
    ns: Vec<Noise>,
}
pub fn fbm1(rng: &Rng, oct: usize) -> Fbm {
    Fbm { ns: (0..oct).map(|_| Noise::new(rng)).collect() }
}
impl Fbm {
    pub fn at(&self, x: f64) -> f64 {
        let (mut v, mut a, mut f) = (0.0, 1.0, 1.0);
        for (k, n) in self.ns.iter().enumerate() {
            v += a * n.at(x * f + 17.0 * (k + 1) as f64);
            a *= 0.5;
            f *= 2.0;
        }
        v
    }
}

// ---------------------------------------------------------------- color
fn srgb_dec(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
fn srgb_enc(c: f64) -> f64 {
    if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}
fn cbrt(x: f64) -> f64 {
    if x < 0.0 { -(-x).powf(1.0 / 3.0) } else { x.powf(1.0 / 3.0) }
}

fn rgb_to_oklch(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (r, g, b) = (srgb_dec(r), srgb_dec(g), srgb_dec(b));
    let l = cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
    let m = cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
    let s = cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
    let ll = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s;
    let aa = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s;
    let bb = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s;
    (ll, (aa * aa + bb * bb).sqrt(), bb.atan2(aa) / RAD)
}

// OKLCH -> sRGB; None when out of gamut
fn oklch_to_rgb(ll: f64, c: f64, h: f64) -> Option<Color> {
    let (a, b) = (c * (h * RAD).cos(), c * (h * RAD).sin());
    let l = (ll + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (ll - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (ll - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let bl = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;
    let out = |x: f64| x < -1e-4 || x > 1.0001;
    if out(r) || out(g) || out(bl) {
        return None;
    }
    let c = |x: f64| srgb_enc(x.clamp(0.0, 1.0));
    Some([c(r), c(g), c(bl)])
}

// largest in-gamut chroma at or below C
fn oklch_clamped(l: f64, c: f64, h: f64) -> Color {
    if let Some(rgb) = oklch_to_rgb(l, c, h) {
        return rgb;
    }
    let (mut lo, mut hi) = (0.0, c);
    for _ in 0..18 {
        let mid = (lo + hi) / 2.0;
        if oklch_to_rgb(l, mid, h).is_some() { lo = mid } else { hi = mid }
    }
    oklch_to_rgb(l, lo, h).unwrap_or([l, l, l])
}

// the ramp: lightness and chroma per stop; chroma scales with the chroma of
// `ink`, the hue follows `ink` and drifts slightly violet in the darks and cyan
// in the lights, as ultramarine does when it is thinned with water
const STOPS: [[f64; 4]; 6] = [
    [0.00, 0.25, 0.105, 6.0],
    [0.18, 0.35, 0.150, 3.0],
    [0.38, 0.47, 0.175, 0.0],
    [0.58, 0.60, 0.145, -3.0],
    [0.80, 0.77, 0.080, -6.0],
    [1.00, 0.92, 0.030, -8.0],
];
const REF_C: f64 = 0.175; // the chroma of the default ink at the cobalt stop

fn build_ramp(hue: f64, chroma_k: f64) -> Vec<Color> {
    (0..=100)
        .map(|i| {
            let t = i as f64 / 100.0;
            let mut k = 0;
            while STOPS[k + 1][0] < t {
                k += 1;
            }
            let (a, b) = (STOPS[k], STOPS[k + 1]);
            let u = (t - a[0]) / (b[0] - a[0]);
            let l = a[1] + (b[1] - a[1]) * u;
            let c = (a[2] + (b[2] - a[2]) * u) * chroma_k;
            let h = hue + a[3] + (b[3] - a[3]) * u;
            oklch_clamped(l, c, h)
        })
        .collect()
}

fn ramp_for(c: Color) -> Vec<Color> {
    let (_, chroma, h) = rgb_to_oklch(c[0], c[1], c[2]);
    build_ramp(h, (chroma / REF_C).clamp(0.0, 1.3))
}

// With a blossom ink (sumi-e: black branches, red flowers) the flowers take
// their washes from a second ramp: what flora's flower primitives paint inside
// in_bloom() at tones above BLOOM_DARK. The darker touches, stamens, eyes,
// outlines and the flowers' own little stems, stay in the ink.
const BLOOM_DARK: f64 = 0.16;

pub const WHITE: Color = [1.0, 1.0, 1.0];
pub const PAPER: Color = [0.995, 0.993, 0.985];

// ---------------------------------------------------------------- state
pub type Transform = [f64; 6]; // x' = a x + c y + e,  y' = b x + d y + f

// While recording (the growth animation) every piece of output keeps the tag
// that was current when it was painted: visible while the progress of event
// `ev` is in [from, to).
#[derive(Clone, Copy, Debug)]
pub struct Tag {
    pub ev: usize,
    pub from: f64,
    pub to: f64,
    pub overlay: bool,
}

impl Tag {
    // visible at progress p of its event; `to` = infinity is open-ended, also
    // for the last frame, whose p is infinite
    pub fn visible(&self, p: f64) -> bool {
        p >= self.from && (self.to == f64::INFINITY || p < self.to)
    }
}

// one painted element: its operators and its tag
type Piece = (String, Option<Tag>);

pub struct Recording {
    layers: BTreeMap<i32, Vec<Piece>>,
}

struct State {
    t: Transform,
    stack: Vec<Transform>,
    layers: BTreeMap<i32, Vec<Piece>>,
    cur: i32,
    tag: Option<Tag>,
    path: String,
    ramp: Vec<Color>,
    accent: Color,
    accent_ramp: Option<(Color, Vec<Color>)>,
    bloom: Option<Vec<Color>>,
    accented: bool,
    blooming: bool,
    lift: f64,
    // the graphics states used: (multiply, opacity in percent)
    gs: BTreeSet<(bool, u32)>,
    bbox: [f64; 4],
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State {
        t: [CM, 0.0, 0.0, CM, 0.0, 0.0],
        stack: vec![],
        layers: BTreeMap::new(),
        cur: 5,
        tag: None,
        path: String::new(),
        ramp: ramp_for([0x24 as f64 / 255.0, 0x56 as f64 / 255.0, 0xC4 as f64 / 255.0]),
        accent: [0.71, 0.40, 0.18],
        accent_ramp: None,
        bloom: None,
        accented: false,
        blooming: false,
        lift: 0.0,
        gs: BTreeSet::new(),
        bbox: [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// a fresh page: identity transform (local cm -> page bp), no output
pub fn reset() {
    with(|s| {
        s.t = [CM, 0.0, 0.0, CM, 0.0, 0.0];
        s.stack.clear();
        s.layers.clear();
        s.cur = 5;
        s.tag = None;
        s.path.clear();
        s.lift = 0.0;
        s.gs.clear();
        s.bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    })
}

// set the hue from the document color `ink` (r,g,b in 0..1)
pub fn set_ink(c: Color) {
    let r = ramp_for(c);
    with(|s| s.ramp = r)
}
pub fn set_accent(c: Color) {
    with(|s| s.accent = c)
}
// the hue of the flowers, or None: the ink's
pub fn set_bloom(c: Option<Color>) {
    let r = c.map(ramp_for);
    with(|s| s.bloom = r)
}

// a point on the ramp (ghost lift applied)
pub fn tone(t: f64) -> Color {
    with(|s| {
        let lt = t + (1.0 - t) * s.lift;
        let i = (lt * 100.0 + 0.5).floor().clamp(0.0, 100.0) as usize;
        match &s.bloom {
            Some(b) if s.blooming && !s.accented && t >= BLOOM_DARK => b[i],
            _ => s.ramp[i],
        }
    })
}
// k>0 lightens toward white, k<0 darkens
pub fn accent(k: f64) -> Color {
    let a = with(|s| s.accent);
    if k >= 0.0 {
        return [a[0] + (1.0 - a[0]) * k, a[1] + (1.0 - a[1]) * k, a[2] + (1.0 - a[2]) * k];
    }
    let k = 1.0 + k;
    [a[0] * k, a[1] * k, a[2] * k]
}
// paint fn() with a ramp at the hue of the accent instead (copper leaves in autumn)
pub fn in_accent(f: impl FnOnce()) {
    let main = with(|s| {
        let a = s.accent;
        let fresh = !matches!(&s.accent_ramp, Some((k, _)) if *k == a);
        if fresh {
            s.accent_ramp = Some((a, ramp_for(a)));
        }
        let ar = s.accent_ramp.as_ref().unwrap().1.clone();
        s.accented = true;
        std::mem::replace(&mut s.ramp, ar)
    });
    f();
    with(|s| {
        s.ramp = main;
        s.accented = false;
    });
}
// paint f() as a flower: in the blossom ink, if there is one
pub fn in_bloom<R>(f: impl FnOnce() -> R) -> R {
    let was = with(|s| std::mem::replace(&mut s.blooming, true));
    let r = f();
    with(|s| s.blooming = was);
    r
}
pub fn set_lift(v: f64) -> f64 {
    with(|s| std::mem::replace(&mut s.lift, v))
}
pub fn get_lift() -> f64 {
    with(|s| s.lift)
}

// ---------------------------------------------------------------- transforms
pub fn set_transform(t: Transform) {
    with(|s| s.t = t)
}
pub fn get_transform() -> Transform {
    with(|s| s.t)
}
pub fn push() {
    with(|s| {
        let t = s.t;
        s.stack.push(t)
    })
}
pub fn pop() {
    with(|s| s.t = s.stack.pop().expect("ink: pop without push"))
}
pub fn translate(x: f64, y: f64) {
    with(|s| {
        let t = s.t;
        s.t = [t[0], t[1], t[2], t[3], t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5]];
    })
}
pub fn rotate(deg: f64) {
    let (c, sn) = ((deg * RAD).cos(), (deg * RAD).sin());
    with(|s| {
        let t = s.t;
        s.t = [t[0] * c + t[2] * sn, t[1] * c + t[3] * sn, -t[0] * sn + t[2] * c, -t[1] * sn + t[3] * c, t[4], t[5]];
    })
}
pub fn scale(sx: f64, sy: f64) {
    with(|s| {
        let t = s.t;
        s.t = [t[0] * sx, t[1] * sx, t[2] * sy, t[3] * sy, t[4], t[5]];
    })
}
pub fn scale1(k: f64) {
    scale(k, k)
}
// size of one local unit on the page, in cm (for level-of-detail decisions)
pub fn unit() -> f64 {
    with(|s| (s.t[0] * s.t[3] - s.t[1] * s.t[2]).abs().sqrt() / CM)
}
// local point -> page cm (for layout code that needs absolute positions)
pub fn to_page(x: f64, y: f64) -> (f64, f64) {
    with(|s| {
        let t = s.t;
        ((t[0] * x + t[2] * y + t[4]) / CM, (t[1] * x + t[3] * y + t[5]) / CM)
    })
}

// ---------------------------------------------------------------- output
pub fn layer(n: i32) -> i32 {
    with(|s| std::mem::replace(&mut s.cur, n))
}
pub fn tag(t: Option<Tag>) -> Option<Tag> {
    with(|s| std::mem::replace(&mut s.tag, t))
}

fn put(piece: String) {
    with(|s| {
        let tag = s.tag;
        s.layers.entry(s.cur).or_default().push((piece, tag));
    })
}

// 0.1 bp (0.035 mm) is finer than any printer resolves
fn p(s: &mut State, x: f64, y: f64) {
    let t = s.t;
    let (px, py) = (t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5]);
    let b = &mut s.bbox;
    b[0] = b[0].min(px);
    b[1] = b[1].min(py);
    b[2] = b[2].max(px);
    b[3] = b[3].max(py);
    tenth(&mut s.path, px);
    s.path.push(' ');
    tenth(&mut s.path, py);
}

// x as "%.1f" would print it. Where x*10 is clearly away from a tie, rounding
// it is exact and much cheaper than float formatting (which dominates the
// plugin's time under Typst's WASM interpreter); near a tie, format! decides.
fn tenth(out: &mut String, x: f64) {
    let t = x * 10.0;
    let r = t.round();
    if (t - r).abs() < 0.4999 && r.abs() < 1e15 {
        let r = r as i64;
        if r < 0 {
            out.push('-');
        } else if x < 0.0 {
            out.push('-'); // -0.04 prints as "-0.0"
        }
        let a = r.unsigned_abs();
        let _ = write!(out, "{}.{}", a / 10, a % 10);
    } else {
        let _ = write!(out, "{:.1}", x);
    }
}
fn sep(s: &mut State) {
    if !s.path.is_empty() {
        s.path.push(' ');
    }
}

// path construction
pub fn moveto(x: f64, y: f64) {
    with(|s| {
        sep(s);
        p(s, x, y);
        s.path.push_str(" m");
    })
}
pub fn lineto(x: f64, y: f64) {
    with(|s| {
        sep(s);
        p(s, x, y);
        s.path.push_str(" l");
    })
}
pub fn curveto(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
    with(|s| {
        sep(s);
        p(s, x1, y1);
        s.path.push(' ');
        p(s, x2, y2);
        s.path.push(' ');
        p(s, x3, y3);
        s.path.push_str(" c");
    })
}
pub fn close() {
    with(|s| {
        sep(s);
        s.path.push('h')
    })
}
pub fn poly(pts: &[Pt], closed: bool) {
    moveto(pts[0][0], pts[0][1]);
    for q in &pts[1..] {
        lineto(q[0], q[1]);
    }
    if closed {
        close();
    }
}
// Catmull-Rom through the points, as Béziers
pub fn smooth_t(pts: &[Pt], closed: bool, tension: f64) {
    let n = pts.len();
    if n < 3 {
        return poly(pts, closed);
    }
    let k = tension / 6.0;
    // 1-based, as in the Lua original
    let at = |i: i64| -> Pt {
        if closed {
            pts[(i - 1).rem_euclid(n as i64) as usize]
        } else {
            pts[(i.clamp(1, n as i64) - 1) as usize]
        }
    };
    moveto(pts[0][0], pts[0][1]);
    let last = if closed { n } else { n - 1 } as i64;
    for i in 1..=last {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        curveto(
            p1[0] + (p2[0] - p0[0]) * k, p1[1] + (p2[1] - p0[1]) * k,
            p2[0] - (p3[0] - p1[0]) * k, p2[1] - (p3[1] - p1[1]) * k,
            p2[0], p2[1],
        );
    }
    if closed {
        close();
    }
}
pub fn smooth(pts: &[Pt], closed: bool) {
    smooth_t(pts, closed, 1.0)
}
pub fn circle(x: f64, y: f64, r: f64) {
    let k = 0.5523 * r;
    moveto(x + r, y);
    curveto(x + r, y + k, x + k, y + r, x, y + r);
    curveto(x - k, y + r, x - r, y + k, x - r, y);
    curveto(x - r, y - k, x - k, y - r, x, y - r);
    curveto(x + k, y - r, x + r, y - k, x + r, y);
    close();
}
fn take() -> String {
    with(|s| std::mem::take(&mut s.path))
}

fn rgb(c: Color, stroke: bool) -> String {
    format!("{:.3} {:.3} {:.3} {}", c[0], c[1], c[2], if stroke { "RG" } else { "rg" })
}
// a graphics state for opacity < 1 or multiply, registered for the page's
// resources; None when the paint is opaque and normal
fn gs(op: f64, mult: bool) -> Option<String> {
    let op = (op * 100.0 + 0.5).floor() as u32;
    if op >= 100 && !mult {
        return None;
    }
    with(|s| s.gs.insert((mult, op)));
    Some(format!("flo{}{}", if mult { "m" } else { "a" }, op))
}
// the ExtGState entries of every graphics state used since reset()
pub fn resources() -> String {
    with(|s| {
        s.gs.iter()
            .map(|&(mult, op)| {
                let a = op as f64 / 100.0;
                format!("/flo{}{} << /ca {a:.2} /CA {a:.2}{} >>", if mult { "m" } else { "a" }, op, if mult { " /BM /Multiply" } else { "" })
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

// paint the current path. opacity < 1 or mult = true wraps it in its own graphics state
pub fn fill(c: Color, op: f64, mult: bool) {
    let g = gs(op, mult);
    let d = take();
    put(match g {
        Some(g) => format!("q /{} gs {} {} f Q", g, rgb(c, false), d),
        None => format!("{} {} f", rgb(c, false), d),
    });
}
pub fn fill1(c: Color) {
    fill(c, 1.0, false)
}
// dash: a PDF dash array, e.g. "[2 2] 0"
pub fn stroke_d(c: Color, w: f64, op: f64, mult: bool, dash: Option<&str>) {
    let g = gs(op, mult);
    let dash = dash.map(|a| format!("{a} d ")).unwrap_or_default();
    let st = format!("{} {:.2} w {}{} S", rgb(c, true), w, dash, take());
    put(match g {
        Some(g) => format!("q /{g} gs {st} Q"),
        None => format!("q {st} Q"),
    });
}
pub fn stroke(c: Color, w: f64, op: f64, mult: bool) {
    stroke_d(c, w, op, mult, None)
}
pub fn stroke1(c: Color, w: f64) {
    stroke_d(c, w, 1.0, false, None)
}
// clip to the current path until the matching unclip()
pub fn clip() {
    let d = take();
    put(format!("q {d} W n"));
}
pub fn unclip() {
    put("Q".to_string())
}
pub fn discard() {
    take();
}
// paint the whole of the current clip (inside a clip(), cheaper than repeating the path)
pub fn flood(c: Color, x0: f64, y0: f64, x1: f64, y1: f64) {
    poly(&[[x0, y0], [x1, y0], [x1, y1], [x0, y1]], true);
    fill1(c);
}

// all layers (in order) as one literal: round caps and joins, as in ink.lua
fn literal<'a>(layers: impl Iterator<Item = Vec<&'a str>>) -> String {
    let parts: Vec<String> = layers.filter(|l| !l.is_empty()).map(|l| l.join("\n")).collect();
    if parts.is_empty() {
        return String::new();
    }
    format!("q 1 J 1 j\n{}\nQ\n", parts.join("\n"))
}

pub fn flush() -> String {
    let layers = with(|s| std::mem::take(&mut s.layers));
    literal(layers.values().map(|l| l.iter().map(|p| p.0.as_str()).collect()))
}

// hand over what was recorded instead of writing it
pub fn take_recording() -> Recording {
    let layers = with(|s| {
        s.tag = None;
        std::mem::take(&mut s.layers)
    });
    Recording { layers }
}
// one frame of a recording: the untagged pieces and those keep(tag) accepts
pub fn emit(rec: &Recording, keep: impl Fn(&Tag) -> bool) -> String {
    literal(rec.layers.values().map(|l| l.iter().filter(|p| p.1.as_ref().map_or(true, &keep)).map(|p| p.0.as_str()).collect()))
}

// the extent of everything drawn since reset(), in page bp: x0, y0, x1, y1
pub fn bbox() -> [f64; 4] {
    with(|s| s.bbox)
}

// ---------------------------------------------------------------- geometry
// outward normals of a polyline
pub fn normals(pts: &[Pt]) -> Vec<Pt> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let mut l = (dx * dx + dy * dy).sqrt();
            if l < 1e-9 {
                l = 1.0
            }
            [-dy / l, dx / l]
        })
        .collect()
}

// cumulative length of a polyline, and the total
pub fn arclen(pts: &[Pt]) -> (Vec<f64>, f64) {
    let mut s = vec![0.0];
    let mut acc = 0.0;
    for i in 1..pts.len() {
        acc += ((pts[i][0] - pts[i - 1][0]).powi(2) + (pts[i][1] - pts[i - 1][1]).powi(2)).sqrt();
        s.push(acc);
    }
    (s, acc)
}

// a ribbon along a centerline: width(u) is the full width at fraction u of the
// length. Returns the closed outline (left side forward, right side back).
pub fn ribbon(pts: &[Pt], width: &dyn Fn(f64) -> f64) -> Vec<Pt> {
    let nrm = normals(pts);
    let (s, l) = arclen(pts);
    let mut left = Vec::with_capacity(pts.len() * 2);
    let mut right = Vec::with_capacity(pts.len());
    for (i, q) in pts.iter().enumerate() {
        let w = width(if l > 0.0 { s[i] / l } else { 0.0 }) / 2.0;
        left.push([q[0] + nrm[i][0] * w, q[1] + nrm[i][1] * w]);
        right.push([q[0] - nrm[i][0] * w, q[1] - nrm[i][1] * w]);
    }
    left.extend(right.into_iter().rev());
    left
}

// a tapered stroke along one cubic Bézier, as a closed outline of two Béziers:
// the control points are pushed out along the curve's normals by half the width
// at their parameter (w0 at the start, w1 at the end)
#[allow(clippy::too_many_arguments)]
pub fn taper(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, w0: f64, w1: f64) {
    let nrm = |dx: f64, dy: f64| {
        let l = (dx * dx + dy * dy).sqrt();
        if l < 1e-9 { (0.0, 0.0) } else { (-dy / l, dx / l) }
    };
    let (n0x, n0y) = nrm(x1 - x0, y1 - y0);
    let (n1x, n1y) = nrm(x2 - x0, y2 - y0);
    let (n2x, n2y) = nrm(x3 - x1, y3 - y1);
    let (n3x, n3y) = nrm(x3 - x2, y3 - y2);
    let (h0, h1, h2, h3) = (w0 / 2.0, (2.0 * w0 + w1) / 6.0, (w0 + 2.0 * w1) / 6.0, w1 / 2.0);
    moveto(x0 + n0x * h0, y0 + n0y * h0);
    curveto(x1 + n1x * h1, y1 + n1y * h1, x2 + n2x * h2, y2 + n2y * h2, x3 + n3x * h3, y3 + n3y * h3);
    lineto(x3 - n3x * h3, y3 - n3y * h3);
    curveto(x2 - n2x * h2, y2 - n2y * h2, x1 - n1x * h1, y1 - n1y * h1, x0 - n0x * h0, y0 - n0y * h0);
    close();
}

// Catmull-Rom samples through control points, `per` per segment
pub fn catmull(pts: &[Pt], per: usize) -> Vec<Pt> {
    let n = pts.len();
    let mut out = vec![];
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (pts[i.saturating_sub(1)], pts[i], pts[i + 1], pts[(i + 2).min(n - 1)]);
        for k in 0..per {
            let t = k as f64 / per as f64;
            let (t2, t3) = (t * t, t * t * t);
            let c = |j: usize| {
                0.5 * (2.0 * p1[j] + (-p0[j] + p2[j]) * t + (2.0 * p0[j] - 5.0 * p1[j] + 4.0 * p2[j] - p3[j]) * t2
                    + (-p0[j] + 3.0 * p1[j] - 3.0 * p2[j] + p3[j]) * t3)
            };
            out.push([c(0), c(1)]);
        }
    }
    out.push(pts[n - 1]);
    out
}

// sample a curve f(v) -> (x, y) at n+1 points, v in [0, 1]
pub fn sample(f: &dyn Fn(f64) -> (f64, f64), n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|i| {
            let (x, y) = f(i as f64 / n as f64);
            [x, y]
        })
        .collect()
}

// a cubic Bézier evaluator, a function of v
#[allow(clippy::too_many_arguments)]
pub fn bezier3(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> impl Fn(f64) -> (f64, f64) {
    move |v| {
        let u = 1.0 - v;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * v, 3.0 * u * v * v, v * v * v);
        (a * x0 + b * x1 + c * x2 + d * x3, a * y0 + b * y1 + c * y2 + d * y3)
    }
}

pub type Spine<'a> = &'a dyn Fn(f64) -> (f64, f64);
pub type Half<'a> = &'a dyn Fn(f64) -> f64;
pub type Edge<'a> = &'a dyn Fn(f64, f64) -> f64;

// a blade: closed outline around a spine with half-widths wl(v), wr(v) on each
// side and an optional edge perturbation e(v, side) added along the normal.
// Used for petals, leaves, sepals, buds. Returns the outline, the centerline
// and its normals.
pub fn blade(spine: Spine, n: usize, wl: Half, wr: Half, edge: Option<Edge>) -> (Vec<Pt>, Vec<Pt>, Vec<Pt>) {
    let c = sample(spine, n);
    let nrm = normals(&c);
    let mut left = Vec::with_capacity(n + 1);
    let mut right = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let v = i as f64 / n as f64;
        let (el, er) = match edge {
            Some(e) => (e(v, 1.0), e(v, -1.0)),
            None => (0.0, 0.0),
        };
        let a = (wl(v) + el).max(0.0);
        let b = (wr(v) + er).max(0.0);
        left.push([c[i][0] + nrm[i][0] * a, c[i][1] + nrm[i][1] * a]);
        right.push([c[i][0] - nrm[i][0] * b, c[i][1] - nrm[i][1] * b]);
    }
    let mut out = left;
    for i in (1..n).rev() {
        out.push(right[i]);
    }
    (out, c, nrm)
}

// an irregular blob: a circle of radius r whose radius wobbles by `rough`
pub fn blob(rng: &Rng, x: f64, y: f64, rx: f64, ry: f64, rough: f64, n: usize) -> Vec<Pt> {
    let (ph, ph2, rot) = (rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI));
    (0..n)
        .map(|i| {
            let a = 2.0 * PI * i as f64 / n as f64;
            let k = 1.0 + rough * (0.6 * (2.0 * a + ph).sin() + 0.4 * (3.0 * a + ph2).sin()) + rough * 0.5 * rng.sym(1.0);
            let (px, py) = (rx * k * a.cos(), ry * k * a.sin());
            [x + px * rot.cos() - py * rot.sin(), y + px * rot.sin() + py * rot.cos()]
        })
        .collect()
}
