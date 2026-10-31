// grow.rs -- the grown composition of florilegium (a port of grow.lua).
//
// A plant, not a packing. Everything grows from one root in the bottom-left
// corner of the page:
//   1. stems: three long stems sway up the margin column (their x is a smooth
//      noise of their height, so they cross now and then); the first ends in the
//      largest flower at the top. A footer stem runs from the root along the
//      bottom of the page. On the last page, extra stems wave into the space
//      under the text.
//   2. leaves are hung at the nodes of the stems, alternating sides.
//   3. a greedy loop in the spirit of Wong, Zongker & Salesin, "Computer-
//      Generated Floral Ornament" (SIGGRAPH 1998): take the largest free disc
//      within reach of a stem, choose a motif that fits it, and grow a branch to
//      it from the nearest stem point. Branches become stems for later branches.
//      Repeat until only small holes are left.
//   4. small holes get florets, buds and leaflets on short stalks; a few copper
//      accents go along the outer edge; pale ghost flowers sit behind.
// Proxies (discs) may overlap a little, so leaves tuck under flowers.
// Everything is deterministic for a given seed.

use crate::flora::{self, O};
use crate::ink::{self, lerp, tone, Pt, Recording, Rng, Tag, RAD};
use std::collections::HashMap;
use std::f64::consts::PI;

// layers, back to front (the overlay only in the growth animation)
pub const L_GHOST: i32 = 1;
pub const L_STEM: i32 = 2;
pub const L_LEAF: i32 = 3;
pub const L_AIRY: i32 = 4;
pub const L_BLOOM: i32 = 5;
pub const L_ACCENT: i32 = 6;
pub const L_OVERLAY: i32 = 7;

const GRID: f64 = 0.2; // candidate spacing (cm)
const REACH: f64 = 3.0; // a motif must be within this distance of a stem
const SMALLEST: f64 = 0.36; // the branch loop stops below this free radius
const FILLMIN: f64 = 0.17; // the filler stops below this

// the growth animation: a stem is recorded as prefixes GROWSTEP cm apart, a
// motif at the scales in BLOOM before its full size (eased: quick, then settling)
const GROWSTEP: f64 = 0.3;
const BLOOM: [f64; 6] = [0.25, 0.45, 0.62, 0.76, 0.87, 0.95];
const FLYSTEPS: usize = 14; // positions of a visitor flying in

fn dist(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}
fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn norm(x: f64, y: f64) -> (f64, f64) {
    let l = (x * x + y * y).sqrt();
    if l < 1e-9 { (0.0, 1.0) } else { (x / l, y / l) }
}
fn deg(x: f64, y: f64) -> f64 {
    y.atan2(x) / RAD
}
fn d() -> O {
    O::default()
}

// ---------------------------------------------------------------- motifs
// r(s): proxy radius; off(s): distance from the disc center back to the
// primitive's origin, along the motif's axis; upright: the axis leans upward
// (sprays reach for the light); draw(rng, s, p): the primitive in a frame whose
// +y is the axis (p: progress of its flight in the animation, None on the page).
// Visitors fly: no branch, and only on pages that the visitor stream allows.
// spring/summer/autumn multiply w as the garden ages (see `season`).
pub struct Motif {
    pub name: &'static str,
    pub w: f64,
    pub smin: f64,
    pub smax: f64,
    pub max: f64,
    pub hard: bool,
    pub layer: i32,
    pub upright: f64,
    pub hang: bool,
    pub fly: bool,
    pub spring: f64,
    pub summer: f64,
    pub autumn: f64,
    pub r: fn(f64) -> f64,
    pub off: fn(f64) -> f64,
    pub draw: fn(&Rng, f64, Option<f64>),
}

const BASE: Motif = Motif {
    name: "",
    w: 1.0,
    smin: 1.0,
    smax: 1.0,
    max: 1.0,
    hard: false,
    layer: L_BLOOM,
    upright: 0.0,
    hang: false,
    fly: false,
    spring: 1.0,
    summer: 1.0,
    autumn: 1.0,
    r: |s| s,
    off: |_| 0.0,
    draw: |_, _, _| {},
};

pub static MOTIFS: [Motif; 14] = [
    Motif { name: "flower", w: 3.0, smin: 0.75, smax: 1.3, max: 3.0, hard: true, layer: L_BLOOM, spring: 0.12, autumn: 0.45,
        r: |s| 1.0 * s, off: |_| 0.0, draw: |rng, s, _| flora::flower(rng, s, &O::default()), ..BASE },
    Motif { name: "lily", w: 2.0, smin: 0.8, smax: 1.25, max: 2.0, hard: true, layer: L_BLOOM, upright: 0.4, spring: 0.1, autumn: 0.4,
        r: |s| 0.9 * s, off: |s| 0.15 * s, draw: |rng, s, _| flora::lily(rng, s, &O::default()), ..BASE },
    Motif { name: "blossoms", w: 3.0, smin: 0.55, smax: 0.85, max: 5.0, layer: L_BLOOM, upright: 0.5, spring: 0.5, autumn: 0.5,
        r: |s| s, off: |s| 0.6 * s, draw: draw_blossoms, ..BASE },
    Motif { name: "cyme", w: 3.0, smin: 1.0, smax: 2.4, max: 6.0, layer: L_AIRY, upright: 0.6, spring: 1.3, autumn: 0.7,
        r: |s| 0.45 * s, off: |s| 0.5 * s, draw: |rng, s, _| flora::cyme(rng, s, &O::default()), ..BASE },
    Motif { name: "sprig", w: 2.0, smin: 1.2, smax: 2.8, max: 4.0, layer: L_AIRY, upright: 0.8, spring: 1.2,
        r: |s| 0.42 * s, off: |s| 0.5 * s, draw: |rng, s, _| flora::sprig(rng, s, &O::default()), ..BASE },
    Motif { name: "umbel", w: 1.5, smin: 0.8, smax: 1.5, max: 3.0, hard: true, layer: L_AIRY, upright: 1.0, spring: 0.8, autumn: 0.8,
        r: |s| 0.75 * s, off: |s| 0.6 * s, draw: |rng, s, _| flora::umbel(rng, s, &O::default()), ..BASE },
    Motif { name: "leaf", w: 2.0, smin: 0.9, smax: 2.0, max: 8.0, layer: L_LEAF, upright: 0.2, spring: 1.3, autumn: 1.2,
        r: |s| 0.32 * s, off: |s| 0.5 * s,
        draw: |rng, s, _| {
            ink::scale1(s);
            let w = rng.range(0.22, 0.3);
            let depth = if rng.chance(0.6) { rng.range(0.4, 0.8) } else { 0.0 };
            flora::leaf(rng, &O { w: Some(w), depth: Some(depth), ..O::default() })
        }, ..BASE },
    Motif { name: "bud", w: 1.0, smin: 0.45, smax: 0.75, max: 5.0, layer: L_BLOOM, upright: 0.5, spring: 3.5, autumn: 0.3,
        r: |s| 0.45 * s, off: |s| 0.5 * s, draw: |rng, s, _| flora::bud(rng, s, &O::default()), ..BASE },
    Motif { name: "bells", w: 0.7, smin: 0.6, smax: 0.95, max: 1.0, hard: true, layer: L_BLOOM, hang: true, spring: 0.5, autumn: 0.6,
        r: |s| 0.62 * s, off: |_| 0.0,
        draw: |rng, s, _| {
            flora::bell(rng, s, &O::default());
            ink::moveto(0.0, 0.0);
            ink::curveto(0.15 * s, 0.25 * s, 0.4 * s, 0.25 * s, 0.5 * s, 0.1 * s);
            ink::stroke1(tone(0.1), 0.35);
            ink::push();
            ink::translate(0.5 * s, 0.1 * s);
            flora::bell(rng, 0.75 * s, &O::default());
            ink::pop();
        }, ..BASE },
    Motif { name: "tendril", w: 0.6, smin: 0.3, smax: 0.5, max: 4.0, layer: L_AIRY, spring: 2.0, autumn: 1.3,
        r: |s| 1.0 * s, off: |s| 1.0 * s,
        draw: |rng, s, _| {
            ink::rotate(90.0);
            flora::tendril(rng, s, &O::default())
        }, ..BASE },
    Motif { name: "budding", w: 3.0, smin: 0.55, smax: 0.9, max: 6.0, layer: L_BLOOM, upright: 0.6, spring: 1.0, summer: 0.0, autumn: 0.0,
        r: |s| 0.9 * s, off: |s| 0.6 * s, draw: draw_budding, ..BASE },
    Motif { name: "seedhead", w: 2.0, smin: 0.45, smax: 0.75, max: 3.0, hard: true, layer: L_AIRY, upright: 0.6, spring: 0.0, summer: 0.0, autumn: 1.0,
        r: |s| 1.05 * s, off: |_| 0.0, draw: |rng, s, _| flora::seedhead(rng, s, &O::default()), ..BASE },
    Motif { name: "butterfly", w: 0.8, smin: 1.1, smax: 1.5, max: 2.0, hard: true, layer: L_ACCENT, fly: true,
        r: |s| 0.5 * s, off: |_| 0.0,
        draw: |rng, s, p| flora::butterfly(rng, s, &O { open: p.map(|p| 0.3 + 0.7 * (p * 5.0 * PI).cos().abs()), ..O::default() }), ..BASE },
    Motif { name: "bee", w: 0.8, smin: 0.7, smax: 0.85, max: 2.0, hard: true, layer: L_ACCENT, fly: true,
        r: |s| 0.6 * s, off: |_| 0.0,
        draw: |rng, s, p| flora::bee(rng, s, &O { trail: Some(p.is_none()), ..O::default() }), ..BASE },
];

// a few small flowers on stalks
fn draw_blossoms(rng: &Rng, s: f64, _: Option<f64>) {
    let n = rng.int(2, 4);
    for k in 1..=n {
        let a = (k as f64 - (n + 1) as f64 / 2.0) * 38.0 + rng.sym(10.0);
        let dd = s * rng.range(0.45, 0.8) * if k == 1 { 1.0 } else { rng.range(0.85, 1.1) };
        let (x, y) = (dd * (a * RAD).sin(), dd * (a * RAD).cos());
        ink::moveto(0.0, 0.0);
        ink::curveto(0.0, dd * 0.4, x * 0.6, y * 0.7, x, y);
        ink::stroke1(tone(0.1), 0.35);
        ink::push();
        ink::translate(x, y);
        flora::blossom(rng, s * rng.range(0.38, 0.5), &O::default());
        ink::pop();
    }
    if rng.chance(0.6) {
        ink::push();
        ink::rotate(rng.sym(50.0));
        ink::scale1(0.5 * s);
        flora::bud(rng, 1.0, &O::default());
        ink::pop();
    }
}

// a spray of closed buds on stalks
fn draw_budding(rng: &Rng, s: f64, _: Option<f64>) {
    let n = rng.int(2, 4);
    for k in 1..=n {
        let a = (k as f64 - (n + 1) as f64 / 2.0) * 34.0 + rng.sym(10.0);
        let dd = s * rng.range(0.5, 0.85);
        let (x, y) = (dd * (a * RAD).sin(), dd * (a * RAD).cos());
        ink::moveto(0.0, 0.0);
        ink::curveto(0.0, dd * 0.4, x * 0.6, y * 0.7, x, y);
        ink::stroke1(tone(0.1), 0.35);
        ink::push();
        ink::translate(x, y);
        ink::rotate(-a * 0.6);
        flora::bud(rng, s * rng.range(0.45, 0.6), &O::default());
        ink::pop();
    }
}

pub fn motif(name: &str) -> Option<&'static Motif> {
    MOTIFS.iter().find(|m| m.name == name)
}

// ---------------------------------------------------------------- input
#[derive(Clone, Debug)]
pub struct Geo {
    pub pw: f64,
    pub ph: f64,
    pub textleft: f64,
    pub textright: f64,
    pub ytop: f64,
    pub ybot: f64,
    pub colright: f64,
    pub headrule: f64,
    pub footy: f64,
}

// a flower the text asks for: beside a heading (level) or \florilegiumbloom[motif]
#[derive(Clone, Debug)]
pub struct Mark {
    pub heading: Option<i64>,
    pub motif: String,
    pub y: f64,
}

pub struct Input {
    pub geo: Geo,
    pub seed: f64,
    pub spill: bool,
    pub spilly: f64,
    pub visitors: bool,
    pub age: Option<f64>,
    pub marks: Vec<Mark>,
    pub anim: bool,
}

pub struct Stats {
    pub cells: usize,
    pub motifs: f64,
    pub visitors: Vec<String>,
}

pub struct Grown {
    pub stats: Stats,
    // the animation's recording and its length in seconds
    pub rec: Option<(Recording, Vec<Event>, f64)>,
}

// ---------------------------------------------------------------- region
// rects = a union of [x0,y0,x1,y1]; shared edges count as open
fn inside(r: &[f64; 4], x: f64, y: f64) -> bool {
    x > r[0] && x < r[2] && y > r[1] && y < r[3]
}
fn border(rects: &[[f64; 4]], x: f64, y: f64) -> f64 {
    let mut best = -1.0;
    for (ri, r) in rects.iter().enumerate() {
        if inside(r, x, y) {
            let mut dd = f64::INFINITY;
            let edges = [[x - r[0], r[0] - 0.05, y], [r[2] - x, r[2] + 0.05, y], [y - r[1], x, r[1] - 0.05], [r[3] - y, x, r[3] + 0.05]];
            for e in &edges {
                let open = rects.iter().enumerate().any(|(oi, o)| oi != ri && inside(o, e[1], e[2]));
                if !open && e[0] < dd {
                    dd = e[0];
                }
            }
            if dd > best {
                best = dd;
            }
        }
    }
    best
}

// ---------------------------------------------------------------- animation timing
// Every step of the growth is an event: a stem that grows at a constant speed,
// or a motif that opens. An event either belongs to a phase (stems, ghost,
// marks, branch, fall, fill, accent, tail) or starts at a fraction `at` of its
// parent's growth, e.g. a leaf when the stem tip passes its node. Times are
// seconds of a nominal timeline; the frames are spread over it.
#[derive(Clone, Default, Debug)]
pub struct Event {
    pub kind: &'static str,
    pub phase: &'static str,
    pub i: usize,
    pub len: Option<f64>,
    pub off: f64,
    pub parent: Option<usize>,
    pub root: usize,
    pub at: f64,
    pub size: Option<f64>,
    pub span: Option<f64>,
    pub till_end: bool,
    pub t0: Option<f64>,
    pub dur: f64,
    pub lead: f64,
}

const T_START: f64 = 0.4; // an empty margin, then the tallest stem takes T_STEMS
const T_STEMS: f64 = 3.6;
const T_BRANCHES: f64 = 6.0; // branch i of n starts at (i/n)^T_EASE: big holes slowly
const T_EASE: f64 = 0.55;
const T_FILL: f64 = 1.2;
const T_ACCENT: f64 = 0.5;

fn schedule(events: &mut [Event]) -> f64 {
    let mut lmax: f64 = 0.0;
    let mut n: HashMap<&str, usize> = HashMap::from([("branch", 0), ("fill", 0), ("accent", 0), ("fall", 0)]);
    for e in events.iter() {
        if e.phase == "stems" {
            lmax = lmax.max(e.len.unwrap_or(0.0));
        }
        if let Some(c) = n.get_mut(e.phase) {
            *c += 1;
        }
    }
    let speed = lmax / T_STEMS;
    let start = |i: usize, nn: usize| T_BRANCHES * ((i as f64 - 1.0) / nn.max(1) as f64).powf(T_EASE);
    let nb = n["branch"];
    for k in 0..events.len() {
        let e = &events[k];
        let mut dur = e.dur;
        let mut lead = e.lead;
        if let Some(span) = e.span {
            dur = span;
        } else if e.kind == "stem" {
            dur = e.len.unwrap_or(0.0) / speed;
        } else if e.kind == "bloom" {
            dur = if events[e.root].phase == "ghost" { 1.2 } else { 0.2 + 0.25 * e.size.unwrap_or(0.5).min(1.0) };
        } else if e.kind == "overlay" {
            // the dashed disc shows a little before its branch starts, longer while the loop is slow
            let i = events[e.parent.unwrap()].i;
            lead = (1.5 * (start(i + 1, nb) - start(i, nb))).clamp(0.08, 0.4);
        }
        let e = &mut events[k];
        e.dur = dur.max(1e-3);
        e.lead = lead;
    }
    // resolve start times in creation order (a parent is always created before its children)
    let resolve = |events: &mut [Event], root_t0: &dyn Fn(&Event) -> Option<f64>| -> f64 {
        let mut last: f64 = 0.0;
        for k in 0..events.len() {
            if events[k].t0.is_none() {
                match events[k].parent {
                    None => {
                        let t = root_t0(&events[k]);
                        events[k].t0 = t;
                    }
                    Some(p) => {
                        let (pt0, pdur) = (events[p].t0, events[p].dur);
                        if let Some(pt0) = pt0 {
                            let e = &mut events[k];
                            if e.kind == "overlay" {
                                e.t0 = Some(pt0 - e.lead);
                                e.dur = e.lead + pdur;
                            } else {
                                e.t0 = Some(pt0 + e.at * pdur);
                            }
                        }
                    }
                }
            }
            let e = &events[k];
            if let (Some(t0), false) = (e.t0, e.till_end) {
                last = last.max(t0 + e.dur);
            }
        }
        last
    };
    let t1 = resolve(events, &|e| match e.phase {
        "stems" => Some(T_START + e.off),
        "ghost" => Some(T_START + 0.8 * e.i as f64),
        _ => None,
    });
    let t2 = resolve(events, &|e| match e.phase {
        "marks" => Some(t1 + 0.1 + 0.3 * (e.i as f64 - 1.0)),
        "branch" => Some(t1 + 0.2 + start(e.i, nb)),
        _ => None,
    });
    let (nfall, nfill, nacc) = (n["fall"], n["fill"], n["accent"]);
    let t3 = resolve(events, &|e| match e.phase {
        "fall" => Some(t2 - 0.4 + T_FILL * (e.i as f64 - 1.0) / nfall.max(1) as f64),
        "fill" => Some(t2 + 0.1 + T_FILL * (e.i as f64 - 1.0) / nfill.max(1) as f64),
        _ => None,
    });
    let t4 = resolve(events, &|e| match e.phase {
        "tail" => Some(t3),
        "accent" => Some(t3 + T_ACCENT * (e.i as f64 - 1.0) / nacc.max(1) as f64),
        _ => None,
    });
    for e in events.iter_mut() {
        let t0 = e.t0.expect("florilegium: an animation event without a start");
        if e.till_end {
            e.dur = (t4 - t0).max(0.5); // e.g. the snail crawls until the end
        }
    }
    t4
}

// the part [0,t] of a cubic Bézier, by de Casteljau
#[allow(clippy::too_many_arguments)]
fn bezier_head(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, t: f64) -> [f64; 6] {
    let (ax, ay) = (lerp(x0, x1, t), lerp(y0, y1, t));
    let (bx, by) = (lerp(x1, x2, t), lerp(y1, y2, t));
    let (cx, cy) = (lerp(x2, x3, t), lerp(y2, y3, t));
    let (dx, dy) = (lerp(ax, bx, t), lerp(ay, by, t));
    let (ex, ey) = (lerp(bx, cx, t), lerp(by, cy, t));
    [ax, ay, dx, dy, lerp(dx, ex, t), lerp(dy, ey, t)]
}

// a flight path for the animation: from distance d in direction `from` (degrees),
// easing out, swinging sideways by `swing`, wobbling about the final heading
fn arrive(x: f64, y: f64, a: f64, d: f64, from: f64, swing: f64) -> impl Fn(f64) -> (f64, f64, f64) {
    let (ux, uy) = ((from * RAD).cos(), (from * RAD).sin());
    move |e| {
        let q = 1.0 - (1.0 - e).powi(2);
        let w = swing * (PI * q).sin();
        (x + d * ux * (1.0 - q) - uy * w, y + d * uy * (1.0 - q) + ux * w, a + 25.0 * (3.0 * PI * e).sin() * (1.0 - q))
    }
}

type Path<'a> = Option<&'a dyn Fn(f64) -> (f64, f64, f64)>;

// ---------------------------------------------------------------- the garden's state
struct Garden {
    rng: Rng,
    layout: Rng,
    anim: bool,
    events: Vec<Event>,
    now: usize,
    gx: Vec<f64>,
    gy: Vec<f64>,
    free: Vec<f64>,
    sdist: Vec<f64>,
    // the global lattice, densely: cell (ix, iy) -> index into gx..., or u32::MAX
    lat: Vec<u32>,
    lx0: i64,
    ly0: i64,
    lw: i64,
    lh: i64,
    maxfree: f64,
    // stem points: x, y, tx, ty, w; a branch may start at any of them
    spts: Vec<[f64; 5]>,
}

impl Garden {
    fn tag(&self, from: f64, to: f64) {
        ink::tag(Some(Tag { ev: self.now, from, to, overlay: false }));
    }
    fn event(&mut self, mut e: Event) -> Option<usize> {
        if !self.anim {
            return None;
        }
        let i = self.events.len();
        e.root = match e.parent {
            Some(p) => self.events[p].root,
            None => i,
        };
        self.events.push(e);
        self.now = i;
        self.tag(0.0, f64::INFINITY);
        Some(i)
    }
    fn cell(&self, ix: i64, iy: i64) -> Option<usize> {
        let (i, j) = (ix - self.lx0, iy - self.ly0);
        if i < 0 || j < 0 || i >= self.lw || j >= self.lh {
            return None;
        }
        let v = self.lat[(i * self.lh + j) as usize];
        if v == u32::MAX { None } else { Some(v as usize) }
    }
    fn near(&self, x: f64, y: f64, r: f64, out: &mut Vec<usize>) {
        out.clear();
        for ix in ((x - r) / GRID).floor() as i64..=((x + r) / GRID).floor() as i64 {
            for iy in ((y - r) / GRID).floor() as i64..=((y + r) / GRID).floor() as i64 {
                if let Some(i) = self.cell(ix, iy) {
                    out.push(i);
                }
            }
        }
    }
    // a proxy disc: cells count it at k of its radius (k < 1 lets things overlap)
    fn disc(&mut self, x: f64, y: f64, r: f64, k: f64) {
        let mut cells = vec![];
        self.near(x, y, r * k + self.maxfree, &mut cells);
        for i in cells {
            let dd = dist(self.gx[i], self.gy[i], x, y) - k * r;
            if dd < self.free[i] {
                self.free[i] = dd;
            }
        }
    }
    fn free_at(&self, x: f64, y: f64) -> f64 {
        match self.cell((x / GRID).floor() as i64, (y / GRID).floor() as i64) {
            Some(i) => self.free[i],
            None => -1.0,
        }
    }
    fn add_stem_points(&mut self, pts: &[Pt], w0: f64, w1: f64, every: usize) {
        let n = pts.len();
        let mut cells = vec![];
        for i in (0..n).step_by(every) {
            let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
            let (tx, ty) = norm(b[0] - a[0], b[1] - a[1]);
            let p = pts[i];
            self.spts.push([p[0], p[1], tx, ty, lerp(w0, w1, i as f64 / (n.max(2) - 1) as f64)]);
            self.near(p[0], p[1], REACH, &mut cells);
            for &j in &cells {
                let dd = dist(self.gx[j], self.gy[j], p[0], p[1]);
                if dd < self.sdist[j] {
                    self.sdist[j] = dd;
                }
            }
        }
    }
    fn nearest_stem(&self, x: f64, y: f64) -> ([f64; 5], f64) {
        let mut best = f64::INFINITY;
        let mut bp = self.spts[0];
        for p in &self.spts {
            let dd = dist(x, y, p[0], p[1]);
            if dd < best {
                best = dd;
                bp = *p;
            }
        }
        (bp, best)
    }
    fn stem(&mut self, pts: &[Pt], w0: f64, w1: f64, proxy: f64) {
        ink::layer(L_STEM);
        if self.anim {
            // growing: the first k/n of its length, for progress in [k/n, (k+1)/n)
            let (s, l) = ink::arclen(pts);
            let n = ((l / GROWSTEP).floor() as usize).max(1);
            let mut j = 0;
            self.events[self.now].len = Some(l);
            for k in 1..n {
                let at = l * k as f64 / n as f64;
                while s[j + 1] < at {
                    j += 1;
                }
                let dd = s[j + 1] - s[j];
                let u = if dd > 1e-9 { (at - s[j]) / dd } else { 0.0 };
                let mut head: Vec<Pt> = pts[..=j].to_vec();
                head.push([lerp(pts[j][0], pts[j + 1][0], u), lerp(pts[j][1], pts[j + 1][1], u)]);
                self.tag(k as f64 / n as f64, (k + 1) as f64 / n as f64);
                flora::stem(&head, w0, lerp(w0, w1, k as f64 / n as f64), None, 1.0, Some(0.4));
            }
            self.tag(1.0, f64::INFINITY);
        }
        flora::stem(pts, w0, w1, None, 1.0, None);
        if self.anim {
            self.tag(0.0, f64::INFINITY);
        }
        for i in (0..pts.len()).step_by(2) {
            self.disc(pts[i][0], pts[i][1], proxy, 1.0);
        }
    }
    // draw a primitive at (x,y) with its +y axis at angle `a` (degrees from +x).
    // In the animation it opens, or with path(e) -> x, y, a it travels to its
    // place in `steps` positions and f gets the progress e.
    #[allow(clippy::too_many_arguments)]
    fn place(&self, layer: i32, x: f64, y: f64, a: f64, f: &dyn Fn(Option<f64>), path: Path, steps: usize) {
        ink::layer(layer);
        if self.anim {
            // a few earlier copies with the same random draws, then the real one
            let (r0, l0) = (self.rng.save(), self.layout.save());
            let n = if path.is_some() { steps } else { BLOOM.len() };
            for k in 1..=n {
                let e = (k - 1) as f64 / n as f64;
                self.tag(e, k as f64 / n as f64);
                ink::push();
                if let Some(path) = path {
                    let (px, py, pa) = path(e);
                    ink::translate(px, py);
                    ink::rotate(pa - 90.0);
                    f(Some(e));
                } else {
                    ink::translate(x, y);
                    ink::rotate(a - 90.0);
                    ink::scale1(BLOOM[k - 1]);
                    f(None);
                }
                ink::pop();
                self.rng.load(r0);
                self.layout.load(l0);
            }
            self.tag(1.0, f64::INFINITY);
        }
        ink::push();
        ink::translate(x, y);
        ink::rotate(a - 90.0);
        f(None);
        ink::pop();
        if self.anim {
            self.tag(0.0, f64::INFINITY);
        }
    }
    fn place0(&self, layer: i32, x: f64, y: f64, a: f64, f: &dyn Fn(Option<f64>)) {
        self.place(layer, x, y, a, f, None, FLYSTEPS)
    }
    // the explained animation marks the free disc a branch grows to, and its stem point
    fn overlay(&mut self, parent: Option<usize>, x: f64, y: f64, r: f64, s: &[f64; 5]) {
        let Some(parent) = parent else { return };
        let e = Event { kind: "overlay", parent: Some(parent), root: self.events[parent].root, ..Default::default() };
        self.events.push(e);
        let old = ink::tag(Some(Tag { ev: self.events.len() - 1, from: 0.0, to: 1.0, overlay: true }));
        ink::layer(L_OVERLAY);
        ink::circle(x, y, r);
        ink::stroke_d(ink::accent(0.0), 0.7, 0.9, false, Some("[2.5 2] 0"));
        ink::circle(x, y, 0.035);
        ink::fill1(ink::accent(0.0));
        ink::circle(s[0], s[1], 0.05);
        ink::fill1(ink::accent(0.0));
        ink::tag(old);
    }
    // a branch from stem point S to a motif at (bx,by) whose axis is (ax,ay)
    #[allow(clippy::too_many_arguments)]
    fn branch_to(&mut self, s: &[f64; 5], bx: f64, by: f64, ax: f64, ay: f64, w0: f64, ev: Option<usize>) -> Vec<Pt> {
        let pts = flora::branch_path(&self.layout, s[0], s[1], s[2], s[3], bx, by, ax, ay);
        let dd = dist(s[0], s[1], bx, by);
        let w = (s[4] * 0.7).min(w0);
        self.stem(&pts, w, (w * 0.45).max(0.012), 0.07);
        self.add_stem_points(&pts, w, (w * 0.45).max(0.012), 2);
        // a leaflet or two on a longer branch
        if dd > 1.0 {
            let nl = if dd > 2.0 { 2 } else { 1 };
            for j in 1..=nl {
                let i = (pts.len() as f64 * (0.25 + 0.3 * j as f64 / nl as f64)).floor() as usize; // 1-based
                let (p, q) = (pts[i - 1], pts[i]);
                let a = deg(q[0] - p[0], q[1] - p[1]) + (if j % 2 == 0 { 1.0 } else { -1.0 }) * self.layout.range(40.0, 60.0);
                let ls = self.layout.range(0.45, 0.75);
                self.event(Event { kind: "bloom", parent: ev, at: i as f64 / pts.len() as f64, size: Some(ls), ..Default::default() });
                self.place0(L_LEAF, p[0], p[1], a, &|_| {
                    ink::scale1(ls);
                    flora::leaflet(&self.rng.fork(), &d())
                });
                self.disc(p[0] + (a * RAD).cos() * ls * 0.5, p[1] + (a * RAD).sin() * ls * 0.5, 0.25 * ls, 0.7);
            }
        }
        pts
    }
}

// the unit tangent of a polyline at (0-based) i
fn tangent(pts: &[Pt], i: usize) -> (f64, f64) {
    let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(pts.len() - 1)]);
    norm(b[0] - a[0], b[1] - a[1])
}

// ---------------------------------------------------------------- page
// The garden for one page, into ink's layers. With g.anim set, the page is
// recorded instead, and the recording is returned with its events timed.
pub fn page(g: &Input) -> Grown {
    let geo = &g.geo;
    let rng = Rng::new(g.seed * 7919.0 + 13.0);
    let layout = rng.fork(); // decisions; primitives get their own forks
    ink::set_transform([ink::CM, 0.0, 0.0, ink::CM, 0.0, 0.0]);
    let anim = g.anim;

    // visitors come from their own stream, so a page without them is unchanged
    let vis = Rng::new(g.seed * 7919.0 + 101.0);
    let (v_butterfly, v_bee, v_snail, v_bird) =
        if g.visitors { (vis.chance(0.75), vis.chance(0.6), vis.chance(0.4), vis.chance(0.22)) } else { (false, false, false, false) };
    let visiting = |name: &str| match name {
        "butterfly" => v_butterfly,
        "bee" => v_bee,
        _ => false,
    };
    // the season (age: 0 spring, 0.5 summer, 1 autumn; None is summer as it
    // always was): a value for each season, blended; its own stream for what it adds
    let age = g.age;
    let season = |sp: f64, su: f64, au: f64| match age {
        None => su,
        Some(a) if a < 0.5 => lerp(sp, su, smoothstep(0.1, 0.5, a)),
        Some(a) => lerp(su, au, smoothstep(0.5, 0.9, a)),
    };
    let (spring, autumn) = (season(1.0, 0.0, 0.0), season(0.0, 0.0, 1.0));
    let sea = Rng::new(g.seed * 7919.0 + 211.0);
    let mut blooms: Vec<[f64; 3]> = vec![]; // the flowers, for the petals that fall in autumn

    // regions: margin column, footer band, and the space under the text on the last page
    let col = [0.3, 0.45, geo.colright, geo.ph - 0.5];
    let fx1 = geo.textleft + (geo.textright - geo.textleft) * layout.range(0.42, 0.52);
    let foot = [geo.colright - 0.01, 0.4, fx1, 1.75f64.min(geo.footy - 0.45)];
    let mut rects = vec![col, foot];
    let mut spill = None;
    if g.spill && (g.spilly - 0.8) - (geo.ybot + 0.6) > 2.5 {
        let r = [geo.colright - 0.01, geo.ybot + 0.6, geo.textright - 0.2, g.spilly - 0.8];
        spill = Some(r);
        rects.push(r);
    }

    // grid on a global lattice, so a disc only has to visit nearby cells
    let fl = |v: f64| (v / GRID).floor() as i64;
    let lx0 = rects.iter().map(|r| fl(r[0])).min().unwrap();
    let ly0 = rects.iter().map(|r| fl(r[1])).min().unwrap();
    let lw = rects.iter().map(|r| fl(r[2])).max().unwrap() - lx0 + 1;
    let lh = rects.iter().map(|r| fl(r[3])).max().unwrap() - ly0 + 1;
    let mut gd = Garden {
        rng,
        layout,
        anim,
        events: vec![],
        now: 0,
        gx: vec![],
        gy: vec![],
        free: vec![],
        sdist: vec![],
        lat: vec![u32::MAX; (lw * lh) as usize],
        lx0,
        ly0,
        lw,
        lh,
        maxfree: 0.0,
        spts: vec![],
    };
    for r in &rects {
        for ix in fl(r[0])..=fl(r[2]) {
            for iy in fl(r[1])..=fl(r[3]) {
                let (x, y) = ((ix as f64 + 0.5) * GRID, (iy as f64 + 0.5) * GRID);
                let k = ((ix - lx0) * lh + (iy - ly0)) as usize;
                if gd.lat[k] == u32::MAX {
                    let b = border(&rects, x, y);
                    if b > 0.0 {
                        gd.lat[k] = gd.gx.len() as u32;
                        gd.gx.push(x);
                        gd.gy.push(y);
                        gd.free.push(b);
                        gd.sdist.push(f64::INFINITY);
                    }
                }
            }
        }
    }
    gd.maxfree = gd.free.iter().cloned().fold(0.0, f64::max);
    let ev = |kind, phase| Event { kind, phase, ..Default::default() };
    let child = |kind, parent: Option<usize>, at: f64, size: Option<f64>| Event { kind, parent, at, size, ..Default::default() };

    // ---------------------------------------------------------- 1. stems
    let root = [gd.layout.range(1.0, 1.6), 0.75];
    let cx = geo.colright * 0.47;
    let amp = geo.colright * 0.2;
    let mut mains: Vec<Vec<Pt>> = vec![];
    let mut mainev: Vec<Option<usize>> = vec![];
    let tops = [geo.ph - 2.6, geo.ph * gd.layout.range(0.6, 0.72), geo.ph * gd.layout.range(0.36, 0.48)];
    for k in 1..=3 {
        let f = ink::fbm1(&gd.layout, 2);
        let (ph, lam) = (gd.layout.range(0.0, 10.0), gd.layout.range(6.0, 9.0));
        let kk = k as f64 - 2.0;
        let x0 = root[0] + kk * 0.25;
        let mut pts = vec![];
        let mut y = root[1];
        while y <= tops[k - 1] {
            let x = cx + amp * f.at(y / lam + ph) + kk * 0.35;
            let x = lerp(x0, x, smoothstep(0.0, 4.0, y - root[1]));
            let x = x.min(geo.colright - 0.9).max(0.9);
            pts.push([x, y]);
            y += 0.25;
        }
        let e = gd.event(ev("stem", "stems"));
        let w0 = if k == 1 { 0.085 } else { 0.065 };
        gd.stem(&pts, w0, 0.028, 0.1);
        gd.add_stem_points(&pts, w0, 0.028, 1);
        mains.push(pts);
        mainev.push(e);
    }
    // the scroll: the root curls into a spiral at the corner
    {
        let turns = gd.layout.range(1.2, 1.6);
        let pts: Vec<Pt> = (0..=40)
            .map(|i| {
                let t = i as f64 / 40.0;
                let a = PI * 0.5 + t * turns * 2.0 * PI;
                let r = 0.55 * (1.0 - 0.85 * t).powf(1.3);
                [root[0] - 0.55 + r * a.cos() * -1.0, root[1] + 0.05 + r * a.sin()]
            })
            .collect();
        gd.event(Event { off: 0.3, ..ev("stem", "stems") });
        gd.stem(&pts, 0.06, 0.012, 0.1);
    }
    // the footer stem: from the root along the bottom, tapering out to a hairline
    let mut footpts: Vec<Pt> = vec![];
    let footev;
    {
        let f = ink::fbm1(&gd.layout, 2);
        let ybase = (foot[1] + foot[3]) / 2.0 - 0.05;
        let mut x = root[0];
        while x <= fx1 - 0.3 {
            let u = smoothstep(root[0], root[0] + 1.5, x);
            let y = lerp(root[1] + 0.1, ybase + 0.22 * f.at(x / 3.5), u);
            footpts.push([x, y]);
            x += 0.2;
        }
        footev = gd.event(Event { off: 0.5, ..ev("stem", "stems") });
        gd.stem(&footpts, 0.065, 0.01, 0.06);
        gd.add_stem_points(&footpts, 0.065, 0.012, 1);
    }
    // on the last page, branches arch out of the main stems into the space
    // under the text, each ending in a flower
    let mut leafy: Vec<(Vec<Pt>, bool, Option<usize>)> = (0..3).map(|k| (mains[k].clone(), true, mainev[k])).collect();
    let mut arcs: Vec<(Vec<Pt>, Option<usize>)> = vec![];
    if let Some(sp) = spill {
        let sh = sp[3] - sp[1];
        let nb = ((sh / 7.0).floor() as usize).max(1);
        for b in 1..=nb {
            let y0 = sp[1] + sh * (b as f64 - 0.75) / nb as f64;
            let si = gd.layout.int(1, 3) as usize - 1;
            let src = &mains[si];
            let mut best = f64::INFINITY;
            let mut bi = 0;
            for (i, p) in src.iter().enumerate() {
                let dd = (p[1] - y0).abs();
                if dd < best {
                    best = dd;
                    bi = i;
                }
            }
            let s0 = src[bi];
            let w = sp[2] - s0[0];
            let x3 = s0[0] + w * gd.layout.range(0.7, 0.88);
            let y3 = (sp[3] - 1.4).min(y0 + sh / nb as f64 * gd.layout.range(0.35, 0.6));
            let c1 = s0[1] + gd.layout.range(-0.4, 0.8);
            let c2 = y3 + gd.layout.range(-1.2, -0.3);
            let mut pts = ink::sample(&ink::bezier3(s0[0], s0[1], s0[0] + w * 0.25, c1, x3 - w * 0.3, c2, x3, y3), (w / 0.2).floor() as usize);
            let f = ink::fbm1(&gd.layout, 2);
            let np = pts.len();
            for (i, p) in pts.iter_mut().enumerate() {
                let u = i as f64 / (np - 1) as f64;
                p[1] += 0.35 * f.at(p[0] / 3.0) * (PI * u).sin();
            }
            let (sa, la) = ink::arclen(src);
            let e = gd.event(child("stem", mainev[si], sa[bi] / la, None));
            gd.stem(&pts, 0.06, 0.025, 0.1);
            gd.add_stem_points(&pts, 0.06, 0.025, 1);
            leafy.push((pts.clone(), true, e));
            arcs.push((pts, e));
        }
    }

    // the header fleuron keeps its spot free
    gd.disc(geo.colright - 0.5, geo.headrule + 0.1, 0.7, 1.0);

    // ---------------------------------------------------------- 2. terminal flowers and ghosts
    let mut counts: HashMap<&'static str, f64> = MOTIFS.iter().map(|m| (m.name, 0.0)).collect();
    {
        let m0 = &mains[0];
        let (p, q) = (m0[m0.len() - 1], m0[m0.len() - 3]);
        let (hx, hy) = norm(p[0] - q[0], p[1] - q[1]);
        let s = 1.75f64.min(geo.colright * 0.27);
        let (fx, fy) = (p[0] + hx * 0.2, p[1] + hy * 0.2);
        gd.event(child("bloom", mainev[0], 1.0, Some(s)));
        gd.place0(L_BLOOM, fx, fy, 90.0, &|_| flora::flower(&gd.rng.fork(), s, &d()));
        gd.disc(fx, fy, 1.05 * s, 0.85);
        blooms.push([fx, fy, s]);
        for k in 1..3 {
            let pts = &mains[k];
            let (p2, q2) = (pts[pts.len() - 1], pts[pts.len() - 3]);
            let (ax, ay) = norm(p2[0] - q2[0], p2[1] - q2[1]);
            // the shorter stems end in a lily and a flower; in spring in buds, in autumn gone to seed
            let tip = if spring > 0.5 { "bud" } else if autumn > 0.5 { "seedhead" } else { "" };
            if k == 1 {
                let s2 = gd.layout.range(1.0, 1.2);
                gd.event(child("bloom", mainev[1], 1.0, Some(s2)));
                match tip {
                    "bud" => gd.place0(L_BLOOM, p2[0], p2[1], deg(ax, ay), &|_| flora::bud(&gd.rng.fork(), 0.95 * s2, &d())),
                    "seedhead" => gd.place0(L_AIRY, p2[0] + ax * 0.15, p2[1] + ay * 0.15, 90.0, &|_| flora::seedhead(&gd.rng.fork(), 0.75 * s2, &d())),
                    _ => gd.place0(L_BLOOM, p2[0] + ax * 0.25 * s2, p2[1] + ay * 0.25 * s2, deg(ax, ay), &|_| flora::lily(&gd.rng.fork(), s2, &d())),
                }
                gd.disc(p2[0] + ax * 0.3 * s2, p2[1] + ay * 0.3 * s2, 0.95 * s2, 0.85);
                *counts.get_mut("lily").unwrap() += 1.0;
            } else {
                let s3 = gd.layout.range(0.85, 1.0);
                gd.event(child("bloom", mainev[2], 1.0, Some(s3)));
                match tip {
                    "bud" => gd.place0(L_BLOOM, p2[0], p2[1], deg(ax, ay), &|_| flora::bud(&gd.rng.fork(), 0.9 * s3, &d())),
                    "seedhead" => gd.place0(L_AIRY, p2[0], p2[1], 90.0, &|_| flora::seedhead(&gd.rng.fork(), 0.75 * s3, &d())),
                    _ => {
                        gd.place0(L_BLOOM, p2[0], p2[1], 90.0, &|_| flora::flower(&gd.rng.fork(), s3, &d()));
                        blooms.push([p2[0], p2[1], s3]);
                    }
                }
                gd.disc(p2[0], p2[1], s3, 0.85);
                *counts.get_mut("flower").unwrap() += 1.0;
            }
        }
    }
    for (pts, e) in &arcs {
        let p = pts[pts.len() - 1];
        let s = gd.layout.range(1.0, 1.25);
        gd.event(child("bloom", *e, 1.0, Some(s)));
        gd.place0(L_BLOOM, p[0], p[1], 90.0, &|_| flora::flower(&gd.rng.fork(), s, &d()));
        gd.disc(p[0], p[1], s, 0.85);
        *counts.get_mut("flower").unwrap() += 1.0;
        blooms.push([p[0], p[1], s]);
    }
    // ghosts: large pale flowers behind, given a little room so they show
    {
        let ng = gd.layout.int(1, 2);
        for k in 1..=ng {
            let gxp = geo.colright * gd.layout.range(0.35, 0.65);
            let gyp = geo.ph * if k == 1 { gd.layout.range(0.15, 0.3) } else { gd.layout.range(0.45, 0.6) };
            let s = gd.layout.range(1.3, 1.6);
            gd.event(Event { i: k as usize, ..ev("bloom", "ghost") });
            gd.place0(L_GHOST, gxp, gyp, 90.0, &|_| flora::ghost(0.62, || flora::flower(&gd.rng.fork(), s, &d())));
            gd.disc(gxp, gyp, 0.8 * s, 0.6);
        }
    }

    // ---------------------------------------------------------- marks: flowers the text asks for
    // beside a heading or wherever a bloom mark stands: the motif sits at the
    // column's edge at the height of that line, on a branch from the nearest
    // stem; where there is no room (the hero flower) it is left out
    let mut nmarks = 0;
    for mk in &g.marks {
        let (m, mut s) = match mk.heading {
            Some(level) => {
                let m = motif(if level == 1 { "flower" } else { "blossoms" });
                (m, if level == 1 { 0.95 } else { 0.7 })
            }
            None => {
                let m = motif(if mk.motif.is_empty() { "flower" } else { &mk.motif });
                (m, m.map_or(0.0, |m| if mk.motif.is_empty() { 0.9 } else { (m.smin + m.smax) / 2.0 }))
            }
        };
        let Some(m) = m else { continue };
        let mut r = (m.r)(s);
        let (px, py) = (geo.colright - r - 0.15, mk.y + 0.12);
        let f = gd.free_at(px, py);
        if f < 0.95 * r {
            let (mut lo, mut hi) = (m.smin * 0.8, s);
            for _ in 0..20 {
                let mid = (lo + hi) / 2.0;
                if (m.r)(mid) > f * 0.95 { hi = mid } else { lo = mid }
            }
            s = lo;
            r = (m.r)(s);
        }
        if r <= f * 1.15 && py > 0.6 + r && py < geo.ph - 0.6 - r {
            let (sp, _) = gd.nearest_stem(px, py);
            let (dx, dy) = norm(px - sp[0], py - sp[1]);
            let (ax, ay) = norm(dx, dy + m.upright);
            let (ox, oy) = (px - ax * (m.off)(s), py - ay * (m.off)(s));
            nmarks += 1;
            let bev = gd.event(Event { i: nmarks, ..ev("stem", "marks") });
            if m.fly {
                gd.event(Event { span: Some(1.2), ..child("bloom", bev, 1.0, None) });
                let path = arrive(px, py, 90.0, 3.0, 200.0, 0.5);
                gd.place(m.layer, px, py, 90.0, &|p| (m.draw)(&gd.rng.fork(), s, p), Some(&path), FLYSTEPS);
            } else {
                gd.branch_to(&sp, ox, oy, ax, ay, 0.045, bev);
                gd.event(child("bloom", bev, 1.0, Some(r)));
                // a heading's flower is painted in the accent, as headings were rubricated
                let heading = mk.heading.is_some();
                gd.place0(m.layer, ox, oy, deg(ax, ay), &|_| {
                    if heading {
                        ink::in_accent(|| (m.draw)(&gd.rng.fork(), s, None))
                    } else {
                        (m.draw)(&gd.rng.fork(), s, None)
                    }
                });
                if m.name == "flower" {
                    blooms.push([ox, oy, s]);
                }
            }
            *counts.get_mut(m.name).unwrap() += 1.0;
            gd.disc(px, py, r, 0.85);
        }
    }

    // visitors on the stems: a snail crawling up a main stem, and now and then
    // a bird on a twig of its own
    if v_snail {
        let s = vis.range(0.75, 0.9);
        let mut best = -1.0;
        let mut c = None;
        for _ in 0..12 {
            let k = vis.int(1, 3) as usize - 1;
            let pts = &mains[k];
            let n = pts.len() as f64;
            let i = vis.int(10.max((n * 0.15).floor() as i64), 11.max((n * 0.45).floor() as i64)) as usize - 1;
            let side = if vis.chance(0.5) { 1.0 } else { -1.0 };
            let (tx, ty) = tangent(pts, i);
            let (nx, ny) = (ty * side, -tx * side); // toward the shell
            let f = gd.free_at(pts[i][0] + nx * 0.8 * s, pts[i][1] + ny * 0.8 * s);
            if f > best {
                best = f;
                c = Some((k, i, side, nx, ny));
            }
        }
        if let (Some((k, i, side, nx, ny)), true) = (c, best > 0.25) {
            let pts = &mains[k];
            let (sa, la) = ink::arclen(pts);
            let i0 = i.saturating_sub(8); // it crawls up the last 2 cm
            let at = |e: f64| {
                let j = lerp(i0 as f64, i as f64, e);
                let j0 = j.floor() as usize;
                let j1 = (j0 + 1).min(pts.len() - 1);
                let (p, q) = (pts[j0], pts[j1]);
                let (tx, ty) = tangent(pts, j0);
                let u = j - j0 as f64;
                (lerp(p[0], q[0], u) + nx * 0.03, lerp(p[1], q[1], u) + ny * 0.03, deg(tx, ty))
            };
            let (x, y, a) = at(1.0);
            gd.event(Event { till_end: true, ..child("bloom", mainev[k], sa[i] / la, None) });
            gd.place(L_ACCENT, x, y, a, &|_| {
                ink::scale(side, 1.0);
                flora::snail(&gd.rng.fork(), s, &d())
            }, Some(&at), 40);
            gd.disc(pts[i][0] + nx * 0.36 * s, pts[i][1] + ny * 0.36 * s, 0.5 * s, 1.0);
            counts.insert("snail", 1.0);
        }
    }
    if v_bird {
        let s = vis.range(2.3, 2.7);
        let mut best = -1.0;
        let mut c = None;
        for _ in 0..24 {
            let k = vis.int(1, 3) as usize - 1;
            let pts = &mains[k];
            let n = pts.len() as f64;
            let i = vis.int((n * 0.25).floor() as i64, (n * 0.75).floor() as i64) as usize - 1;
            let p = pts[i];
            let dir = if vis.chance(0.5) { 1.0 } else { -1.0 };
            let l = vis.range(1.2, 1.7);
            let (ex, ey) = (p[0] + dir * l, p[1] + vis.range(0.05, 0.4));
            let (bx, by) = (ex - dir * 0.25, ey);
            let facing = if vis.chance(0.65) { dir } else { -dir };
            let (cx, cy) = (bx + facing * 0.03 * s, by + 0.4 * s);
            let f = (gd.free_at(cx, cy) / (0.36 * s)).min(gd.free_at(bx - facing * 0.35 * s, by + 0.15 * s) / (0.2 * s));
            if f > best {
                best = f;
                c = Some((k, i, ex, ey, dir, facing, cx, cy));
            }
        }
        if let (Some((k, i, ex, ey, dir, facing, cx, cy)), true) = (c, best > 1.0) {
            let pts = mains[k].clone();
            let p = pts[i];
            let (tx, ty) = tangent(&pts, i);
            let (sa, la) = ink::arclen(&pts);
            let twig = flora::branch_path(&vis, p[0], p[1], tx, ty, ex, ey, dir, 0.1);
            let tev = gd.event(child("stem", mainev[k], sa[i] / la, None));
            gd.stem(&twig, 0.05, 0.02, 0.07);
            gd.add_stem_points(&twig, 0.05, 0.02, 2);
            let perch = twig[((twig.len() as f64 * 0.85).floor() as usize).max(1) - 1];
            gd.event(Event { span: Some(1.4), ..child("bloom", tev, 1.0, None) });
            let path = arrive(perch[0], perch[1], 90.0, 3.5, if facing > 0.0 { 160.0 } else { 20.0 }, 0.6);
            gd.place(L_ACCENT, perch[0], perch[1], 90.0, &|_| flora::bird(&gd.rng.fork(), s, &O { facing: Some(facing), ..d() }), Some(&path), FLYSTEPS);
            gd.disc(cx, cy, 0.36 * s, 0.9);
            gd.disc(perch[0] - facing * 0.35 * s, perch[1] + 0.15 * s, 0.2 * s, 0.9);
            counts.insert("bird", 1.0);
        }
    }

    // ---------------------------------------------------------- 3. leaves at the nodes
    // big stems get full leaves, alternating sides, smaller toward the top; the
    // footer gets leaflets that thin out toward its end
    leafy.push((footpts.clone(), false, footev));
    for (pts, big, sev) in &leafy {
        let big = *big;
        let mut s = 0.0;
        let mut acc = gd.layout.range(0.6, 1.3);
        let mut side = if gd.layout.chance(0.5) { 1.0 } else { -1.0 };
        let mut total = 0.0;
        for i in 1..pts.len() {
            total += dist(pts[i][0], pts[i][1], pts[i - 1][0], pts[i - 1][1]);
        }
        for i in 1..pts.len().saturating_sub(4) {
            s += dist(pts[i][0], pts[i][1], pts[i - 1][0], pts[i - 1][1]);
            if s < acc {
                continue;
            }
            let u = s / total;
            acc += if big { gd.layout.range(1.4, 2.2) } else { lerp(0.55, 1.4, u) * gd.layout.range(0.8, 1.2) };
            side = -side;
            let (tx, ty) = norm(pts[i + 1][0] - pts[i - 1][0], pts[i + 1][1] - pts[i - 1][1]);
            let a = deg(tx, ty) + side * gd.layout.range(35.0, 60.0);
            let (ux, uy) = ((a * RAD).cos(), (a * RAD).sin());
            let mut lf = if big { lerp(2.4, 1.3, pts[i][1] / geo.ph) * gd.layout.range(0.85, 1.1) } else { lerp(0.95, 0.45, u) * gd.layout.range(0.85, 1.1) };
            lf *= season(0.8, 1.0, 1.0); // young leaves are small
            let mut ok = false;
            for _ in 0..3 {
                let (mx, my) = (pts[i][0] + ux * lf * 0.55, pts[i][1] + uy * lf * 0.55);
                if gd.free_at(mx, my) >= 0.24 * lf {
                    ok = true;
                    break;
                }
                lf *= 0.75;
            }
            if ok && lf > if big { 0.7 } else { 0.3 } {
                gd.event(child("bloom", *sev, u, Some(0.4 * lf)));
                if big {
                    let lobed = gd.layout.chance(0.6);
                    let bend = side * gd.layout.range(0.05, 0.3);
                    // young leaves are pale; in autumn some turn copper
                    let copper = autumn > 0.0 && sea.chance(0.35 * autumn);
                    gd.place0(L_LEAF, pts[i][0], pts[i][1], a, &|_| {
                        let leaf = || {
                            ink::scale1(lf);
                            let r = gd.rng.fork();
                            let w = gd.layout.range(0.22, 0.3);
                            let depth = if lobed { gd.layout.range(0.4, 0.8) } else { 0.0 };
                            flora::leaf(&r, &O { w: Some(w), depth: Some(depth), bend: Some(bend), ..d() })
                        };
                        if copper {
                            ink::in_accent(leaf)
                        } else if spring > 0.0 {
                            flora::ghost(0.3 * spring, leaf)
                        } else {
                            leaf()
                        }
                    });
                } else {
                    let bend = -side * gd.layout.range(0.1, 0.3);
                    gd.place0(L_LEAF, pts[i][0], pts[i][1], a, &|_| {
                        ink::scale1(lf);
                        flora::leaflet(&gd.rng.fork(), &O { bend: Some(bend), ..d() })
                    });
                }
                for f in [0.3, 0.55, 0.8] {
                    gd.disc(pts[i][0] + ux * lf * f, pts[i][1] + uy * lf * f, 0.24 * lf, 0.85);
                }
            }
        }
    }

    // ---------------------------------------------------------- 4. branches to the largest holes
    let area: f64 = rects.iter().map(|r| (r[2] - r[0]) * (r[3] - r[1])).sum();
    let af = (area / 175.0).max(1.0);
    let caps: HashMap<&str, f64> = MOTIFS.iter().map(|m| (m.name, if m.fly { m.max } else { (m.max * af + 0.5).floor() })).collect();
    let mut nbranch = 0;
    for _ in 0..400 {
        let mut best = -1.0;
        let mut bi = None;
        for i in 0..gd.gx.len() {
            if gd.sdist[i] <= REACH {
                let v = gd.free[i] * gd.layout.range(0.85, 1.15);
                if v > best {
                    best = v;
                    bi = Some(i);
                }
            }
        }
        let Some(bi) = bi else { break };
        if gd.free[bi] < SMALLEST {
            break;
        }
        let (f, px, py) = (gd.free[bi], gd.gx[bi], gd.gy[bi]);
        // statement motifs have a hard cap; the others only lose weight as they repeat
        let mut elig: Vec<(&Motif, f64)> = vec![];
        let mut wsum = 0.0;
        for m in MOTIFS.iter() {
            let wt = m.w * season(m.spring, m.summer, m.autumn) / (1.0 + counts[m.name] / caps[m.name].max(1.0));
            let ok = wt > 0.0 && (m.r)(m.smin) <= f * 1.05 && (!m.hard || counts[m.name] < caps[m.name]) && (!m.fly || visiting(m.name));
            if ok {
                elig.push((m, wt));
                wsum += wt;
            }
        }
        if elig.is_empty() {
            gd.free[bi] = 0.0;
        } else {
            let mut pick = elig[elig.len() - 1].0;
            let mut acc = gd.layout.u() * wsum;
            for &(m, wt) in &elig {
                acc -= wt;
                if acc <= 0.0 {
                    pick = m;
                    break;
                }
            }
            let mut s = pick.smax;
            if (pick.r)(s) > f * 0.95 {
                let (mut lo, mut hi) = (pick.smin, s);
                for _ in 0..20 {
                    let mid = (lo + hi) / 2.0;
                    if (pick.r)(mid) > f * 0.95 { hi = mid } else { lo = mid }
                }
                s = lo;
            }
            let (sp, _) = gd.nearest_stem(px, py);
            let (dx, dy) = norm(px - sp[0], py - sp[1]);
            // the motif's axis: away from the stem, leaning up for the upright kinds
            let (ax, ay) = norm(dx, dy + pick.upright);
            let (mut ox, mut oy) = (px - ax * (pick.off)(s), py - ay * (pick.off)(s));
            if pick.hang {
                ox = px - dx * 0.2 * s;
                oy = py + 0.45 * s;
            }
            nbranch += 1;
            let bev = gd.event(Event { i: nbranch, ..ev("stem", "branch") });
            gd.overlay(bev, px, py, f, &sp);
            if pick.fly {
                // a visitor: no branch, any heading, and in the animation it flies in
                let a = if pick.name == "bee" { gd.layout.range(0.0, 360.0) } else { 90.0 + gd.layout.sym(50.0) };
                gd.event(Event { span: Some(1.2), ..child("bloom", bev, 1.0, None) });
                let from = gd.layout.range(0.0, 360.0);
                let swing = gd.layout.sym(0.8);
                let path = arrive(px, py, a, 3.0, from, swing);
                gd.place(pick.layer, px, py, a, &|p| (pick.draw)(&gd.rng.fork(), s, p), Some(&path), FLYSTEPS);
            } else {
                if dist(ox, oy, sp[0], sp[1]) > 0.12 {
                    let (bax, bay) = if pick.hang { (dx, -0.3) } else { (ax, ay) };
                    gd.branch_to(&sp, ox, oy, bax, bay, 0.04, bev);
                }
                let a = if pick.hang { 90.0 } else { deg(ax, ay) };
                gd.event(child("bloom", bev, 1.0, Some((pick.r)(s))));
                gd.place0(pick.layer, ox, oy, a, &|_| (pick.draw)(&gd.rng.fork(), s, None));
                if pick.name == "flower" {
                    blooms.push([ox, oy, s]);
                }
            }
            *counts.get_mut(pick.name).unwrap() += 1.0;
            gd.disc(px, py, (pick.r)(s), 0.85);
        }
    }

    // autumn: loose petals fall from the flowers into the free space below them
    // (in the animation they drift down from the flower)
    if autumn > 0.0 {
        let mut nfall = 0;
        for b in &blooms {
            for _ in 0..sea.int(0, (3.0 * autumn + 0.5).floor() as i64) {
                let mut best = 0.16;
                let mut at = None;
                for _ in 0..6 {
                    let (x, y) = (b[0] + sea.sym(1.1), b[1] - b[2] - sea.range(0.1, 2.2));
                    let f = gd.free_at(x, y);
                    if f > best {
                        best = f;
                        at = Some((x, y));
                    }
                }
                if let Some((bx, by)) = at {
                    let (ps, pa, pt) = (sea.range(0.4, 0.6), sea.range(0.0, 360.0), sea.range(0.36, 0.5));
                    let (sway, x0, y0) = (sea.sym(0.6), b[0] + sea.sym(0.3), b[1] - 0.3 * b[2]);
                    nfall += 1;
                    gd.event(Event { i: nfall, span: Some(1.6), ..ev("bloom", "fall") });
                    let path = move |e: f64| (lerp(x0, bx, e) + sway * (2.0 * PI * e).sin(), lerp(y0, by, e * e * (3.0 - 2.0 * e)), pa - 140.0 * (1.0 - e));
                    gd.place(L_BLOOM, bx, by, pa, &|_| {
                        ink::scale1(ps);
                        flora::petal(&gd.rng.fork(), &O { tone: Some(pt), heart: Some(false), ..d() });
                    }, Some(&path), 16);
                    gd.disc(bx, by, 0.22, 1.0);
                }
            }
        }
    }

    // ---------------------------------------------------------- 5. small holes
    // only right next to a stem, and not too many: a pair of leaflets at a node,
    // a bud, or a few florets. Far from any stem, a copper accent or nothing.
    let (mut accents, mut fills) = (0, 0);
    let maxfill = (28.0 * af).floor() as usize;
    for _ in 0..600 {
        let mut best = -1.0;
        let mut bi = None;
        for i in 0..gd.gx.len() {
            let v = gd.free[i] * gd.layout.range(0.85, 1.15);
            if v > best {
                best = v;
                bi = Some(i);
            }
        }
        let Some(bi) = bi else { break };
        if gd.free[bi] < FILLMIN {
            break;
        }
        let (f, px, py) = (gd.free[bi], gd.gx[bi], gd.gy[bi]);
        let (sp, sd) = gd.nearest_stem(px, py);
        if sd < 0.75 && fills < maxfill {
            fills += 1;
            gd.event(Event { i: fills, size: Some(f), ..ev("bloom", "fill") });
            let (dx, dy) = norm(px - sp[0], py - sp[1]);
            let (ax, ay) = norm(dx, dy + 0.4);
            let kind = gd.layout.u();
            if kind < 0.4 {
                // two leaflets in a V at the stem point
                let base = deg(ax, ay);
                let ls = 0.75f64.min(f * 1.6);
                for k in [-1.0, 1.0] {
                    let a = base + k * gd.layout.range(20.0, 35.0);
                    gd.place0(L_LEAF, sp[0], sp[1], a, &|_| {
                        ink::scale1(ls * gd.layout.range(0.8, 1.0));
                        flora::leaflet(&gd.rng.fork(), &O { bend: Some(-k * 0.15), ..d() })
                    });
                }
            } else if kind < 0.65 {
                let (ox, oy) = (px - ax * f * 0.5, py - ay * f * 0.5);
                ink::layer(L_STEM);
                ink::moveto(sp[0], sp[1]);
                ink::curveto(sp[0] + sp[2] * 0.15, sp[1] + sp[3] * 0.15, ox - ax * 0.1, oy - ay * 0.1, ox, oy);
                ink::stroke1(tone(0.1), 0.4);
                gd.place0(L_BLOOM, ox, oy, deg(ax, ay), &|_| flora::bud(&gd.rng.fork(), 0.6f64.min(f * 1.7), &d()));
            } else {
                // a small cluster of florets on hairline stalks
                let n = gd.layout.int(2, 4);
                for _ in 0..n {
                    let (qx, qy) = (px + gd.layout.sym(f * 0.5), py + gd.layout.sym(f * 0.5));
                    ink::layer(L_STEM);
                    ink::moveto(sp[0], sp[1]);
                    ink::curveto(sp[0] + sp[2] * 0.1, sp[1] + sp[3] * 0.1, (sp[0] + qx) / 2.0, (sp[1] + qy) / 2.0 + 0.05, qx, qy);
                    ink::stroke(tone(0.12), 0.25, 0.9, false);
                    gd.place0(L_BLOOM, qx, qy, 90.0, &|_| {
                        let r = gd.rng.fork();
                        flora::floret(&r, 0.11f64.min(f * 0.6) * gd.layout.range(0.8, 1.1), &d())
                    });
                }
            }
        } else if sd >= 0.75 && accents < 6 && f > 0.22 && px < geo.colright {
            accents += 1;
            gd.event(Event { i: accents, size: Some(0.3), ..ev("bloom", "accent") });
            if gd.layout.chance(0.6) {
                gd.place0(L_ACCENT, px, py, 90.0, &|_| flora::sparkle(&gd.rng.fork(), 0.13, &d()));
            } else {
                gd.place0(L_ACCENT, px, py, 90.0, &|_| flora::rosette(&gd.rng.fork(), 0.12, &d()));
            }
        }
        gd.disc(px, py, f.max(0.3), 1.0);
    }

    // ---------------------------------------------------------- 6. the footer's tail
    {
        let e = footpts[footpts.len() - 1];
        let x2 = e[0] + gd.layout.range(1.6, 2.2);
        ink::layer(L_ACCENT);
        let c = [e[0], e[1], e[0] + 0.6, e[1] + 0.05, x2 - 0.6, e[1] - 0.05, x2, e[1]];
        let tail = gd.event(Event { len: Some(x2 - e[0]), ..ev("stem", "tail") });
        if anim {
            for k in 1..=7 {
                gd.tag(k as f64 / 8.0, (k + 1) as f64 / 8.0);
                ink::moveto(c[0], c[1]);
                let h = bezier_head(c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], k as f64 / 8.0);
                ink::curveto(h[0], h[1], h[2], h[3], h[4], h[5]);
                ink::stroke(ink::accent(0.0), 0.3, 0.8, false);
            }
            gd.tag(1.0, f64::INFINITY);
        }
        ink::moveto(c[0], c[1]);
        ink::curveto(c[2], c[3], c[4], c[5], c[6], c[7]);
        ink::stroke(ink::accent(0.0), 0.3, 0.8, false);
        gd.event(child("bloom", tail, 1.0, Some(0.3)));
        gd.place0(L_ACCENT, x2 + 0.18, e[1], 90.0, &|_| flora::sparkle(&gd.rng.fork(), 0.12, &O { rays: Some(false), ..d() }));
        ink::circle(x2 + 0.45, e[1], 0.025);
        ink::fill1(ink::accent(0.0));
    }

    // assemble: a safety clip a little outside the regions, then the layers
    ink::tag(None);
    ink::layer(0);
    for (i, r) in rects.iter().enumerate() {
        let xr = r[2] + if i == 0 { 0.25 } else { 0.3 };
        ink::poly(&[[r[0] - 0.25, r[1] - 0.3], [xr, r[1] - 0.3], [xr, r[3] + 0.3], [r[0] - 0.25, r[3] + 0.3]], true);
    }
    ink::clip();
    ink::layer(99);
    ink::unclip();

    let stats = Stats {
        cells: gd.gx.len(),
        motifs: counts.values().sum(),
        visitors: ["butterfly", "bee", "snail", "bird"]
            .iter()
            .filter_map(|n| counts.get(n).filter(|&&c| c > 0.0).map(|c| format!("{c} {n}")))
            .collect(),
    };
    let rec = if anim {
        let total = schedule(&mut gd.events);
        Some((ink::take_recording(), gd.events, total))
    } else {
        None
    };
    Grown { stats, rec }
}
