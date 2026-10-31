// sumi.rs -- the plum branch: a margin in the manner of a sumi-e painting.
//
// Black ink branches, gnarled and angular, enter the margin from the edge of
// the page and fork into straight young shoots; small five-petal blossoms,
// buds and half-open flowers sit along them in a second ink (vermilion), a
// few petals fall. No leaves, and empty paper between. A branch is painted as
// a dry brush would leave it: a pale gray body, and over it a few dark
// strands (bristles) that break off in places, with moss dots on the old wood.
//
// The branches grow as zigzag polylines: every segment turns, alternately, a
// little toward a goal direction, and turns back toward the middle of its
// region where it would leave it; a branch stops where it runs into another.
// Children leave at the joints, at 25-60 degrees.

use crate::grow::{Geo, Mark, L_ACCENT, L_BLOOM, L_GHOST, L_LEAF, L_STEM};
use crate::ink::{self, lerp, tone, Pt, Rng, RAD};
use std::f64::consts::PI;

pub struct Input {
    pub geo: Geo,
    pub seed: f64,
    pub spill: bool,
    pub spilly: f64,
    pub age: Option<f64>,
    pub marks: Vec<Mark>,
}

struct Branch {
    pts: Vec<Pt>,
    ws: Vec<f64>, // full width at each joint (cm)
    depth: usize,
    tip: bool, // ends in a point (not cut off by the page edge)
}

fn adiff(a: f64, b: f64) -> f64 {
    (a - b + 180.0).rem_euclid(360.0) - 180.0
}
fn allowed(rects: &[[f64; 4]], x: f64, y: f64, inset: f64) -> bool {
    rects.iter().any(|r| x > r[0] + inset && x < r[2] - inset && y > r[1] + inset && y < r[3] - inset)
}
// the middle of the region that holds (x, y), or of the first
fn middle(rects: &[[f64; 4]], x: f64, y: f64) -> (f64, f64) {
    let r = rects.iter().find(|r| x > r[0] && x < r[2] && y > r[1] && y < r[3]).unwrap_or(&rects[0]);
    ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0)
}
fn near(occ: &[Pt], x: f64, y: f64, d: f64) -> bool {
    occ.iter().any(|p| (p[0] - x).powi(2) + (p[1] - y).powi(2) < d * d)
}

struct Tree<'a> {
    rng: &'a Rng,
    rects: Vec<[f64; 4]>,
    occ: Vec<Pt>, // points along every branch so far
    out: Vec<Branch>,
}

impl Tree<'_> {
    // points every 1 mm along a polyline, for the crowding test
    fn occupy(&mut self, pts: &[Pt]) {
        for i in 1..pts.len() {
            let (p, q) = (pts[i - 1], pts[i]);
            let n = (((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).sqrt() / 0.1).ceil() as usize;
            for k in 0..n {
                let t = k as f64 / n as f64;
                self.occ.push([lerp(p[0], q[0], t), lerp(p[1], q[1], t)]);
            }
        }
    }

    // a twig from (x0, y0) to a mark at (tx, ty): zigzag segments aimed at it,
    // `w` cm wide at its start, and a side twig at some of its joints
    fn reach(&mut self, x0: f64, y0: f64, tx: f64, ty: f64, w: f64, depth: usize) {
        let r = self.rng;
        let (mut x, mut y) = (x0, y0);
        let mut pts = vec![[x, y]];
        let mut angs = vec![];
        let mut zig = if r.chance(0.5) { 1.0 } else { -1.0 };
        loop {
            let d = ((tx - x).powi(2) + (ty - y).powi(2)).sqrt();
            let seg = if depth <= 1 { r.range(0.6, 1.2) } else { r.range(0.4, 0.8) };
            if d <= seg * 1.3 {
                pts.push([tx, ty]);
                angs.push((ty - y).atan2(tx - x) / RAD);
                break;
            }
            if r.chance(0.7) {
                zig = -zig;
            }
            let a = (ty - y).atan2(tx - x) / RAD + zig * r.range(8.0, 30.0);
            (x, y) = (x + seg * (a * RAD).cos(), y + seg * (a * RAD).sin());
            pts.push([x, y]);
            angs.push(a);
        }
        let (at, l) = ink::arclen(&pts);
        let ws: Vec<f64> = at.iter().map(|&t| (w * lerp(1.0, 0.35, t / l)).max(0.012)).collect();
        self.occupy(&pts);
        self.out.push(Branch { pts: pts.clone(), ws: ws.clone(), depth, tip: false });
        for i in 1..pts.len() - 1 {
            if r.chance(0.45) {
                let ca = angs[i - 1] + if r.chance(0.5) { 1.0 } else { -1.0 } * r.range(25.0, 55.0);
                self.grow(pts[i][0], pts[i][1], ca, ca + adiff(90.0, ca) * 0.3, r.range(0.6, 1.6), (ws[i] * 0.55).max(0.02), depth + 1, false);
            }
        }
    }

    // a branch from (x0, y0) heading a0 toward goal (degrees), `len` cm long,
    // `w` cm wide at its start; then its children
    #[allow(clippy::too_many_arguments)]
    fn grow(&mut self, x0: f64, y0: f64, a0: f64, goal: f64, len: f64, w: f64, depth: usize, straight: bool) {
        let r = self.rng;
        let turn = if straight { 9.0 } else { [30.0, 26.0, 20.0, 12.0][depth.min(3)] };
        let (mut x, mut y, mut a, mut goal) = (x0, y0, a0, goal);
        let mut pts = vec![[x, y]];
        let mut angs = vec![a];
        let mut at = vec![0.0];
        let mut entered = allowed(&self.rects, x, y, 0.0);
        let mut s = 0.0;
        let mut zig = if r.chance(0.5) { 1.0 } else { -1.0 };
        let mut cut = false;
        while s < len - 0.05 {
            let seg = if straight {
                r.range(0.5, 1.0)
            } else {
                match depth {
                    0 => r.range(0.55, 1.1),
                    1 => r.range(0.4, 0.85),
                    _ => r.range(0.3, 0.7),
                }
            }
            .min(len - s);
            zig = -zig;
            a += zig * r.range(0.3, 1.0) * turn + r.sym(turn * 0.3);
            a += adiff(goal, a) * 0.35;
            // at the edge of the region a twig ends; a limb turns toward the
            // middle and keeps its new heading
            let mut step = None;
            for _ in 0..6 {
                let (nx, ny) = (x + seg * (a * RAD).cos(), y + seg * (a * RAD).sin());
                if entered && !allowed(&self.rects, nx, ny, 0.12) {
                    if depth > 0 {
                        break;
                    }
                    let (cx, cy) = middle(&self.rects, x, y);
                    let to = (cy - y).atan2(cx - x) / RAD;
                    a += adiff(to, a).signum() * 30.0;
                    goal = a;
                    continue;
                }
                step = Some((nx, ny));
                break;
            }
            let Some((nx, ny)) = step else { break };
            // run into another branch: stop (a child near its parent's start is let through)
            if s > 0.35 && near(&self.occ, nx, ny, 0.22 + w * 0.5) {
                break;
            }
            entered |= allowed(&self.rects, nx, ny, 0.0);
            (x, y) = (nx, ny);
            s += seg;
            pts.push([x, y]);
            angs.push(a);
            at.push(s);
            if !entered && s > 3.0 {
                cut = true;
                break;
            }
        }
        if pts.len() < 2 || cut {
            return;
        }
        let tipk = if depth == 0 { 0.3 } else { 0.18 };
        let ws: Vec<f64> = at.iter().map(|&t| (w * lerp(1.0, tipk, t / s)).max(0.012)).collect();
        self.occupy(&pts);
        self.out.push(Branch { pts: pts.clone(), ws: ws.clone(), depth, tip: true });
        if depth >= 3 || straight {
            return;
        }
        // children at the joints, and on the older wood straight young shoots
        let p = [0.75, 0.55, 0.35][depth];
        for i in 1..pts.len() - 1 {
            if r.chance(p) {
                let side = if r.chance(0.5) { 1.0 } else { -1.0 };
                let ca = angs[i] + side * r.range(25.0, 60.0);
                let rest = s - at[i];
                let cl = rest * r.range(0.45, 0.85) + if depth == 0 { r.range(0.5, 2.0) } else { 0.2 };
                let cw = (ws[i] * r.range(0.4, 0.65)).max(0.02);
                let goal = ca + adiff(90.0, ca) * 0.3;
                self.grow(pts[i][0], pts[i][1], ca, goal, cl, cw, depth + 1, false);
            }
            if depth <= 1 && r.chance(0.16) {
                let ca = 90.0 + r.sym(40.0);
                self.grow(pts[i][0], pts[i][1], ca, ca, r.range(0.8, 2.0), 0.03, 3, true);
            }
        }
    }
}

// ---------------------------------------------------------------- painting
// a number in [0, 1) from a point, so the bow of a segment needs no rng draw
// (the spine is computed again where the blossoms are placed)
fn hash(p: Pt, k: f64) -> f64 {
    ((p[0] * 12.9898 + p[1] * 78.233 + k * 37.719).sin() * 43758.5453).rem_euclid(1.0)
}

// the centerline of a branch: angular at the joints (Catmull-Rom through the
// joints alone would make the zigzag a wave), and between them a stroke that
// bows a little to one side, sometimes with an S, never ruled straight;
// sampled every few millimeters
fn spine(pts: &[Pt]) -> Vec<Pt> {
    let mut dense = vec![pts[0]];
    for i in 1..pts.len() {
        let (p, q) = (pts[i - 1], pts[i]);
        let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
        // bow and S as fractions of the length; the normal is (-dy, dx)/len,
        // so multiplying by dx, dy gives the offset in cm
        let bow = (if hash(p, 1.0) < 0.5 { 1.0 } else { -1.0 }) * lerp(0.05, 0.12, hash(q, 2.0));
        let ess = (hash(p, 3.0) - 0.5) * 0.08;
        for t in [0.15, 0.32, 0.5, 0.68, 0.85, 1.0] {
            let o = bow * (PI * t).sin() + ess * (2.0 * PI * t).sin();
            dense.push([lerp(p[0], q[0], t) - dy * o, lerp(p[1], q[1], t) + dx * o]);
        }
    }
    ink::catmull(&dense, 2)
}

// per point of `c`: the width, interpolated between the joints of `b`
fn widths(rng: &Rng, b: &Branch, c: &[Pt]) -> Vec<f64> {
    let (sj, lj) = ink::arclen(&b.pts);
    let (sc, lc) = ink::arclen(c);
    let noise = ink::fbm1(rng, 2);
    let ph = rng.range(0.0, 50.0);
    sc.iter()
        .map(|&s| {
            let u = s / lc.max(1e-9);
            let t = u * lj;
            let mut k = 0;
            while k + 2 < sj.len() && sj[k + 1] < t {
                k += 1;
            }
            let f = ((t - sj[k]) / (sj[k + 1] - sj[k]).max(1e-9)).clamp(0.0, 1.0);
            let mut w = lerp(b.ws[k], b.ws[k + 1], f) * (1.0 + 0.25 * noise.at(ph + s * 1.4) + 0.1 * noise.at(ph + 31.0 + s * 9.0));
            if b.tip {
                w *= ((1.0 - u) / 0.12).clamp(0.0, 1.0).powf(0.6);
            }
            w.max(0.004)
        })
        .collect()
}

// the outline of a band along c[i0..i1]: center offset off[i], width w[i]
fn band(c: &[Pt], nrm: &[Pt], off: &[f64], w: &[f64], i0: usize, i1: usize) -> Vec<Pt> {
    let mut out: Vec<Pt> = (i0..i1).map(|i| [c[i][0] + nrm[i][0] * (off[i] + w[i] / 2.0), c[i][1] + nrm[i][1] * (off[i] + w[i] / 2.0)]).collect();
    out.extend((i0..i1).rev().map(|i| [c[i][0] + nrm[i][0] * (off[i] - w[i] / 2.0), c[i][1] + nrm[i][1] * (off[i] - w[i] / 2.0)]));
    out
}

// cut the points 0..n (arc lengths sc) into runs of `run` cm, with gaps of
// `gap` cm between them where `chance` says so; runs keep at least 2 points
fn runs(rng: &Rng, sc: &[f64], run: (f64, f64), gap: (f64, f64), chance: f64) -> Vec<(usize, usize)> {
    let n = sc.len();
    let mut out = vec![];
    let (mut i0, mut s) = (0, 0.0);
    while i0 + 1 < n {
        s += rng.range(run.0, run.1);
        let mut i1 = i0 + 2;
        while i1 < n && sc[i1 - 1] < s {
            i1 += 1;
        }
        let i1 = i1.min(n);
        out.push((i0, i1));
        if i1 >= n {
            break;
        }
        // a gap (flying white), or an overlap of one point
        i0 = if rng.chance(chance) {
            s += rng.range(gap.0, gap.1);
            let mut j = i1;
            while j < n && sc[j] < s {
                j += 1;
            }
            j
        } else {
            i1 - 1
        };
        s = s.max(sc[i0.min(n - 1)]);
    }
    out.retain(|&(a, b)| b >= a + 2);
    out
}

// pointed ends where a run starts or stops inside the branch
fn ends(ws: &[f64], i0: usize, i1: usize, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let from = if i0 == 0 { 9.0 } else { (i - i0.min(i)) as f64 };
            let to = if i1 == n { 9.0 } else { (i1 - 1).saturating_sub(i) as f64 };
            ws[i] * (from.min(to) / 3.0).clamp(0.25, 1.0)
        })
        .collect()
}

fn paint_branch(rng: &Rng, b: &Branch) {
    let c = spine(&b.pts);
    let nrm = ink::normals(&c);
    let w = widths(rng, b, &c);
    let (sc, _) = ink::arclen(&c);
    let n = c.len();
    let w0 = b.ws[0];
    let zero = vec![0.0; n];
    // a pale bleed beside parts of the old wood
    if w0 > 0.1 {
        ink::layer(L_GHOST);
        let wb: Vec<f64> = w.iter().map(|v| v * 1.3 + 0.04).collect();
        let off: Vec<f64> = w.iter().map(|v| v * 0.12).collect();
        for (i0, i1) in runs(rng, &sc, (0.8, 2.5), (0.5, 2.0), 0.8) {
            ink::poly(&band(&c, &nrm, &off, &ends(&wb, i0, i1, n), i0, i1), true);
            ink::fill(tone(0.6), rng.range(0.1, 0.2), true);
        }
    }
    ink::layer(L_STEM);
    let strands = ((w0 / 0.05) as usize).clamp(1, 7);
    // the body: washes of uneven strength, gray where the brush ran dry
    if strands >= 2 {
        for (i0, i1) in runs(rng, &sc, (0.6, 2.2), (0.0, 0.0), 0.0) {
            ink::poly(&band(&c, &nrm, &zero, &ends(&w, i0, i1, n), i0, i1), true);
            ink::fill(tone(rng.range(0.06, 0.35)), rng.range(0.6, 0.85), true);
        }
    }
    // the bristles: dark strands across the width, broken (flying white)
    for k in 0..strands {
        let base = if strands == 1 { 0.0 } else { ((k as f64 + 0.5) / strands as f64 - 0.5) * 0.7 + rng.sym(0.2) };
        let wander = ink::fbm1(rng, 2);
        let ph = rng.range(0.0, 50.0);
        let wk = if strands == 1 { 1.0 } else { rng.range(1.3, 2.2) / strands as f64 };
        let off: Vec<f64> = (0..n).map(|i| w[i] * (base * 0.85 + 0.08 * wander.at(ph + sc[i] * 2.0))).collect();
        let ws: Vec<f64> = w.iter().map(|v| v * wk).collect();
        let pieces = if strands >= 3 { runs(rng, &sc, (0.5, 3.0), (0.04, 0.25), 0.6) } else { vec![(0, n)] };
        for (i0, i1) in pieces {
            ink::poly(&band(&c, &nrm, &off, &ends(&ws, i0, i1, n), i0, i1), true);
            let op = if strands == 1 { 0.92 } else { rng.range(0.6, 0.9) };
            ink::fill(tone(rng.range(0.0, 0.07)), op, true);
        }
    }
    // moss dots on the old wood, in twos and threes along the edges
    if b.depth <= 1 && w0 > 0.07 {
        ink::layer(L_LEAF);
        let (_, l) = ink::arclen(&c);
        for _ in 0..(l * 0.5) as usize {
            let i = rng.int(1, n as i64 - 2) as usize;
            let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
            for _ in 0..rng.int(1, 3) {
                let d = side * w[i] / 2.0 * rng.range(0.7, 1.15);
                let (x, y) = (c[i][0] + nrm[i][0] * d + rng.sym(0.06), c[i][1] + nrm[i][1] * d + rng.sym(0.06));
                let rr = rng.range(0.015, 0.04);
                ink::poly(&ink::blob(rng, x, y, rr, rr * rng.range(0.6, 1.0), 0.2, 7), true);
                ink::fill(tone(0.0), 0.9, false);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Open,
    Half,
    Bud,
}

// a plum blossom at the origin, axis +y (away from the branch); r = radius (cm)
fn blossom(rng: &Rng, r: f64, kind: Kind) {
    ink::in_bloom(|| {
        let t0 = rng.range(0.32, 0.6);
        match kind {
            Kind::Open => {
                let rot = rng.range(0.0, 72.0);
                for k in 0..5 {
                    let a = (rot + 72.0 * k as f64 + rng.sym(8.0)) * RAD;
                    let d = 0.5 * r * rng.range(0.9, 1.1);
                    ink::smooth(&ink::blob(rng, d * a.cos(), d * a.sin(), 0.5 * r, 0.45 * r, 0.12, 9), true);
                    ink::fill(tone(t0 + rng.sym(0.07)), rng.range(0.6, 0.85), true);
                }
            }
            Kind::Half => {
                for k in -1..=1 {
                    let a = (90.0 + 48.0 * k as f64 + rng.sym(8.0)) * RAD;
                    let d = 0.45 * r;
                    ink::smooth(&ink::blob(rng, d * a.cos(), 0.15 * r + d * a.sin(), 0.45 * r, 0.4 * r, 0.12, 9), true);
                    ink::fill(tone(t0 + rng.sym(0.07)), rng.range(0.75, 0.9), true);
                }
            }
            Kind::Bud => {
                ink::smooth(&ink::blob(rng, 0.0, 0.35 * r, 0.36 * r, 0.42 * r, 0.08, 8), true);
                ink::fill(tone(t0 - 0.05), 0.9, true);
            }
        }
    });
    // the dark touches: stamens of an open flower, the calyx of the others
    match kind {
        Kind::Open => {
            ink::circle(0.0, 0.0, 0.1 * r);
            ink::fill(tone(0.1), 0.7, true);
            if r > 0.12 {
                for _ in 0..rng.int(5, 8) {
                    let a = rng.range(0.0, 2.0 * PI);
                    let l = r * rng.range(0.35, 0.6);
                    ink::moveto(0.0, 0.0);
                    ink::lineto(l * a.cos(), l * a.sin());
                    ink::stroke(tone(0.04), 0.25, 0.8, false);
                    ink::circle(l * a.cos(), l * a.sin(), 0.045 * r);
                    ink::fill(tone(0.0), 0.9, false);
                }
            }
        }
        _ => {
            ink::smooth(&ink::blob(rng, 0.0, 0.0, 0.2 * r, 0.14 * r, 0.15, 7), true);
            ink::fill(tone(0.02), 0.9, false);
        }
    }
}

struct Flowers {
    at: Vec<[f64; 3]>, // x, y, r
}

impl Flowers {
    // a blossom at (x, y) facing `a` (degrees), unless it would crowd another
    #[allow(clippy::too_many_arguments)]
    fn put(&mut self, rng: &Rng, rects: &[[f64; 4]], x: f64, y: f64, a: f64, r: f64, kind: Kind) -> bool {
        if !allowed(rects, x, y, 0.05) || self.at.iter().any(|f| (f[0] - x).powi(2) + (f[1] - y).powi(2) < (0.75 * (f[2] + r)).powi(2)) {
            return false;
        }
        self.at.push([x, y, r]);
        ink::layer(L_BLOOM);
        ink::push();
        ink::translate(x, y);
        ink::rotate(a - 90.0);
        blossom(&rng.fork(), r, kind);
        ink::pop();
        true
    }
}

pub fn page(g: &Input) {
    let geo = &g.geo;
    let rng = Rng::new(g.seed * 7919.0 + 17.0);
    let layout = rng.fork();
    ink::set_transform([ink::CM, 0.0, 0.0, ink::CM, 0.0, 0.0]);
    let age = g.age.unwrap_or(0.5).clamp(0.0, 1.0);

    // regions: the margin column, and on the last page the space under the text
    let mut rects = vec![[0.3, 0.6, geo.colright, geo.ph - 0.3]];
    if g.spill && (g.spilly - 0.8) - (geo.ybot + 0.6) > 2.5 {
        rects.push([geo.colright - 0.01, geo.ybot + 0.6, geo.textright - 0.2, g.spilly - 0.8]);
    }
    let mut tree = Tree { rng: &layout, rects: rects.clone(), occ: vec![], out: vec![] };

    // the limbs: one from the left edge near the top, hanging down; one lower,
    // rising; often one from the bottom; on the last page one into the spill
    let ph = geo.ph;
    tree.grow(-0.4, ph - layout.range(0.8, 2.6), layout.range(-25.0, -5.0), -72.0, layout.range(13.0, 17.0), layout.range(0.5, 0.65), 0, false);
    tree.grow(-0.4, ph * layout.range(0.3, 0.45), layout.range(15.0, 40.0), 50.0, layout.range(7.0, 10.0), layout.range(0.34, 0.45), 0, false);
    if layout.chance(0.7) {
        tree.grow(layout.range(0.4, 1.6), 0.0, layout.range(60.0, 80.0), 78.0, layout.range(6.0, 9.0), layout.range(0.32, 0.42), 0, false);
    }
    if rects.len() > 1 {
        let r = rects[1];
        tree.grow(-0.4, layout.range(r[1] + 0.5, r[3] - 0.5), layout.range(-10.0, 15.0), 0.0, r[2] - r[0] + 3.0, layout.range(0.3, 0.4), 0, false);
    }

    // twigs to the marks: from the nearest wood to the column's edge at the mark
    let mut asked: Vec<(f64, f64, usize)> = vec![];
    for m in &g.marks {
        let (tx, ty) = (geo.colright - 0.4, m.y);
        let best = tree.occ.iter().map(|p| (((p[0] - tx).powi(2) + (p[1] - ty).powi(2)).sqrt(), *p)).min_by(|a, b| a.0.total_cmp(&b.0));
        let (d, p) = best.unwrap_or((f64::INFINITY, [0.0, 0.0]));
        if d > 2.5 {
            // no wood near: a branch in from the page's edge
            tree.reach(-0.4, ty + layout.sym(1.2), tx, ty, layout.range(0.24, 0.32), 1);
        } else if d > 0.3 {
            tree.reach(p[0], p[1], tx, ty, 0.05, 2);
        }
        asked.push((tx, ty, if m.heading == Some(1) { 5 } else { 3 }));
    }

    for b in &tree.out {
        paint_branch(&layout.fork(), b);
    }

    // blossoms: at the marks first, then along the wood (sparse on the old
    // limbs, crowded on the twigs), a bud at most twig tips
    let mut fl = Flowers { at: vec![] };
    let bud = lerp(0.55, 0.12, age);
    let pick = |r: &Rng| {
        let u = r.u();
        if u < bud { Kind::Bud } else if u < bud + 0.25 { Kind::Half } else { Kind::Open }
    };
    for &(x, y, n) in &asked {
        let mut got = fl.put(&layout, &rects, x, y, 90.0 + layout.sym(30.0), layout.range(0.22, 0.28), Kind::Open) as usize;
        for _ in 0..n * 4 {
            if got >= n {
                break;
            }
            let a = layout.range(0.0, 2.0 * PI);
            let d = layout.range(0.3, 0.6);
            got += fl.put(&layout, &rects, x + d * a.cos(), y + d * a.sin(), a / RAD, layout.range(0.15, 0.22), pick(&layout)) as usize;
        }
    }
    for b in &tree.out {
        let c = spine(&b.pts);
        let nrm = ink::normals(&c);
        let (sc, l) = ink::arclen(&c);
        let density = [0.2, 0.65, 0.95, 1.0][b.depth.min(3)];
        let mut s = layout.range(0.1, 0.4);
        let mut i = 0;
        while s < l - 0.1 {
            while i + 1 < c.len() && sc[i + 1] < s {
                i += 1;
            }
            if layout.chance(density) {
                let k = if layout.chance(0.6) { 1 } else if layout.chance(0.75) { 2 } else { 3 };
                for _ in 0..k {
                    let side = if layout.chance(0.5) { 1.0 } else { -1.0 };
                    let r = layout.range(0.15, 0.27);
                    let kind = pick(&layout);
                    let w = b.ws[0] * lerp(1.0, 0.25, s / l);
                    let d = side * (w / 2.0 + r * layout.range(0.0, 0.6));
                    let (x, y) = (c[i][0] + nrm[i][0] * d + layout.sym(0.08), c[i][1] + nrm[i][1] * d + layout.sym(0.08));
                    let a = (nrm[i][1] * side).atan2(nrm[i][0] * side) / RAD + layout.sym(35.0);
                    fl.put(&layout, &rects, x, y, a, if kind == Kind::Bud { r * 0.75 } else { r }, kind);
                }
            }
            s += layout.range(0.22, 0.55);
        }
        if b.tip && b.depth >= 2 && layout.chance(0.7) {
            let (p, q) = (c[c.len() - 2], c[c.len() - 1]);
            fl.put(&layout, &rects, q[0], q[1], (q[1] - p[1]).atan2(q[0] - p[0]) / RAD, layout.range(0.1, 0.14), Kind::Bud);
        }
    }

    // a few petals falling, more late in the season
    let falls = layout.int(1, 3) + (age * 4.0) as i64;
    for _ in 0..falls {
        if fl.at.is_empty() {
            break;
        }
        let f = fl.at[layout.int(0, fl.at.len() as i64 - 1) as usize];
        let (x, y) = (f[0] + layout.sym(1.0), f[1] - layout.range(0.8, 3.5));
        if allowed(&rects, x, y, 0.1) && !near(&tree.occ, x, y, 0.3) {
            ink::layer(L_ACCENT);
            ink::in_bloom(|| {
                ink::smooth(&ink::blob(&layout.fork(), x, y, 0.08, 0.05, 0.15, 8), true);
                ink::fill(tone(layout.range(0.3, 0.55)), 0.75, true);
            });
        }
    }

    // assemble: a safety clip a little outside the regions (the limbs come in
    // from beyond the page's edge), then the layers
    ink::layer(0);
    for (i, r) in rects.iter().enumerate() {
        let x0 = if i == 0 { -1.0 } else { r[0] - 0.25 };
        let y1 = if i == 0 { ph + 1.0 } else { r[3] + 0.3 };
        ink::poly(&[[x0, r[1] - 1.0], [r[2] + 0.25, r[1] - 1.0], [r[2] + 0.25, y1], [x0, y1]], true);
    }
    ink::clip();
    ink::layer(99);
    ink::unclip();
}

// A headpiece: a plum spray in a band `w` x `h` cm, a limb coming in from
// the left edge (from the right with `flip`) and reaching across, often a
// second shorter one beside it. For the space above a chapter's title.
pub fn spray(w: f64, h: f64, seed: f64, age: f64, flip: bool) {
    let rng = Rng::new(seed * 7919.0 + 29.0);
    let layout = rng.fork();
    ink::set_transform(if flip { [-ink::CM, 0.0, 0.0, ink::CM, w * ink::CM, 0.0] } else { [ink::CM, 0.0, 0.0, ink::CM, 0.0, 0.0] });
    let age = age.clamp(0.0, 1.0);
    let rects = vec![[0.2, 0.25, w - 0.3, h - 0.45]];
    let mut tree = Tree { rng: &layout, rects: rects.clone(), occ: vec![], out: vec![] };
    let y0 = h * layout.range(0.4, 0.7);
    tree.grow(-0.4, y0, layout.range(-15.0, 15.0), layout.range(-8.0, 8.0), w * layout.range(0.55, 0.8), layout.range(0.3, 0.4), 0, false);
    if layout.chance(0.6) {
        let y1 = if y0 > h / 2.0 { h * layout.range(0.15, 0.35) } else { h * layout.range(0.7, 0.85) };
        tree.grow(-0.4, y1, layout.range(-10.0, 25.0), layout.range(0.0, 20.0), w * layout.range(0.2, 0.35), layout.range(0.18, 0.25), 1, false);
    }
    for b in &tree.out {
        paint_branch(&layout.fork(), b);
    }

    // blossoms along the wood, as on the page, and a bud at most twig tips
    let mut fl = Flowers { at: vec![] };
    let bud = lerp(0.55, 0.12, age);
    let pick = |r: &Rng| {
        let u = r.u();
        if u < bud { Kind::Bud } else if u < bud + 0.25 { Kind::Half } else { Kind::Open }
    };
    for b in &tree.out {
        let c = spine(&b.pts);
        let nrm = ink::normals(&c);
        let (sc, l) = ink::arclen(&c);
        let density = [0.2, 0.65, 0.95, 1.0][b.depth.min(3)];
        let mut s = layout.range(0.1, 0.4);
        let mut i = 0;
        while s < l - 0.1 {
            while i + 1 < c.len() && sc[i + 1] < s {
                i += 1;
            }
            if layout.chance(density) {
                let k = if layout.chance(0.6) { 1 } else { 2 };
                for _ in 0..k {
                    let side = if layout.chance(0.5) { 1.0 } else { -1.0 };
                    let r = layout.range(0.15, 0.25);
                    let kind = pick(&layout);
                    let w = b.ws[0] * lerp(1.0, 0.25, s / l);
                    let d = side * (w / 2.0 + r * layout.range(0.0, 0.6));
                    let (x, y) = (c[i][0] + nrm[i][0] * d + layout.sym(0.08), c[i][1] + nrm[i][1] * d + layout.sym(0.08));
                    let a = (nrm[i][1] * side).atan2(nrm[i][0] * side) / RAD + layout.sym(35.0);
                    fl.put(&layout, &rects, x, y, a, if kind == Kind::Bud { r * 0.75 } else { r }, kind);
                }
            }
            s += layout.range(0.25, 0.6);
        }
        if b.tip && b.depth >= 1 && layout.chance(0.7) {
            let (p, q) = (c[c.len() - 2], c[c.len() - 1]);
            fl.put(&layout, &rects, q[0], q[1], (q[1] - p[1]).atan2(q[0] - p[0]) / RAD, layout.range(0.1, 0.14), Kind::Bud);
        }
    }

    // a petal or two drifting down and away
    for _ in 0..layout.int(1, 2) + (age * 2.0) as i64 {
        if fl.at.is_empty() {
            break;
        }
        let f = fl.at[layout.int(0, fl.at.len() as i64 - 1) as usize];
        let (x, y) = (f[0] + layout.range(0.3, 1.5), f[1] - layout.range(0.4, 1.4));
        if allowed(&rects, x, y, 0.1) && !near(&tree.occ, x, y, 0.3) {
            ink::layer(L_ACCENT);
            ink::in_bloom(|| {
                ink::smooth(&ink::blob(&layout.fork(), x, y, 0.08, 0.05, 0.15, 8), true);
                ink::fill(tone(layout.range(0.3, 0.55)), 0.75, true);
            });
        }
    }
}

// the header: a dry-brush rule from the margin to the text's right edge, and
// a red seal under its right end, its strokes cut on a 3x3 grid
pub fn head(g: &Geo, rng: &Rng) {
    ink::set_transform([ink::CM, 0.0, 0.0, ink::CM, 0.0, 0.0]);
    ink::layer(100);
    let (x0, x1, y) = (g.colright - 0.1, g.textright, g.headrule);
    for k in 0..3 {
        let o = (k as f64 - 1.0) * 0.012;
        let e = x1 - rng.range(0.0, 1.5) * (k as f64);
        ink::taper(x0, y + o, lerp(x0, e, 0.3), y + o + rng.sym(0.01), lerp(x0, e, 0.7), y + o + rng.sym(0.01), e, y + o, 0.035, 0.008);
        ink::fill(tone(0.04), rng.range(0.5, 0.8), true);
    }
    let (s, cx, cy) = (0.4, x1 - 0.22, y - 0.32);
    ink::push();
    ink::translate(cx, cy);
    ink::rotate(rng.sym(3.0));
    let h = s / 2.0;
    let edge: Vec<Pt> = (0..16)
        .map(|i| {
            let t = (i % 4) as f64 / 4.0;
            let (x, y) = match i / 4 {
                0 => (lerp(-h, h, t), -h),
                1 => (h, lerp(-h, h, t)),
                2 => (lerp(h, -h, t), h),
                _ => (-h, lerp(h, -h, t)),
            };
            [x + rng.sym(0.008), y + rng.sym(0.008)]
        })
        .collect();
    ink::poly(&edge, true);
    ink::stroke(ink::accent(0.0), 1.1, 0.9, true);
    let q = s * 0.28;
    for _ in 0..rng.int(3, 5) {
        let (i, j) = (rng.int(-1, 1) as f64, rng.int(-1, 1) as f64);
        let (di, dj) = if rng.chance(0.5) { (1.0, 0.0) } else { (0.0, 1.0) };
        let (a, b) = ((i - di * 0.5).clamp(-1.0, 1.0), (j - dj * 0.5).clamp(-1.0, 1.0));
        let (c, d) = ((i + di * 0.5).clamp(-1.0, 1.0), (j + dj * 0.5).clamp(-1.0, 1.0));
        ink::moveto(a * q, b * q);
        ink::lineto(c * q, d * q);
        ink::stroke(ink::accent(0.0), 1.3, 0.9, true);
    }
    ink::pop();
}
