// authored.rs -- the authored composition of florilegium (a port of authored.lua).
//
// Everything here is placed by hand, for A4 with a 68 mm left margin (the
// margin column is x in [0.3, 6.4] cm, the page 29.7 cm tall): three stems
// through hand-picked control points, leaves at chosen fractions of their
// length, and motifs at chosen points, each joined to a chosen stem by a
// branch. Only the parameters are sampled (every petal, lobe and vein, and a
// jitter of a millimeter or two on the control points), so every page gets the
// same garden, differently painted. The grown composition (grow.rs) uses the
// same primitives and the same motif table.

use crate::flora::{self, O};
use crate::grow::{self, Geo, L_ACCENT, L_GHOST, L_LEAF, L_STEM};
use crate::ink::{self, Pt, Rng, RAD};
use std::f64::consts::PI;

fn norm(x: f64, y: f64) -> (f64, f64) {
    let l = (x * x + y * y).sqrt();
    if l < 1e-9 { (0.0, 1.0) } else { (x / l, y / l) }
}
fn deg(x: f64, y: f64) -> f64 {
    y.atan2(x) / RAD
}

// ---------------------------------------------------------------- the score
const ROOT: Pt = [1.35, 0.75];

struct Stem {
    name: &'static str,
    pts: &'static [Pt],
    w: [f64; 2],
    tip: (&'static str, f64, f64),
}

// stems: control points (cm), base and tip width, what grows at the tip
const STEMS: [Stem; 3] = [
    Stem { name: "A", pts: &[[1.35, 0.75], [2.3, 3.2], [1.7, 6.6], [2.9, 10.2], [2.1, 13.8], [3.2, 17.6], [2.3, 21.2], [2.9, 24.4], [2.7, 25.6]],
        w: [0.085, 0.03], tip: ("flower", 1.7, 90.0) },
    Stem { name: "B", pts: &[[1.55, 0.75], [3.1, 4.2], [3.7, 8.3], [2.7, 12.2], [3.9, 15.8], [4.4, 18.4]],
        w: [0.065, 0.028], tip: ("lily", 1.15, 70.0) },
    Stem { name: "C", pts: &[[1.15, 0.75], [0.95, 3.6], [2.1, 7.3], [1.3, 10.6], [1.4, 12.0]],
        w: [0.06, 0.028], tip: ("flower", 0.95, 90.0) },
];

// leaves at the nodes: stem, fraction of its length, side (+1 left), length, lobed
const LEAVES: [(&str, f64, f64, f64, bool); 14] = [
    ("A", 0.08, -1.0, 2.3, true), ("A", 0.20, 1.0, 2.2, false), ("A", 0.33, -1.0, 2.0, true), ("A", 0.47, 1.0, 1.9, true),
    ("A", 0.60, -1.0, 1.8, false), ("A", 0.72, 1.0, 1.6, true), ("A", 0.84, -1.0, 1.4, false),
    ("B", 0.15, 1.0, 2.1, true), ("B", 0.33, -1.0, 1.9, false), ("B", 0.52, 1.0, 1.8, true), ("B", 0.72, -1.0, 1.5, true),
    ("C", 0.22, 1.0, 1.8, false), ("C", 0.46, -1.0, 1.7, true), ("C", 0.70, 1.0, 1.4, false),
];

// motifs on branches: stem, fraction where the branch leaves, target center (cm), motif, size
const BRANCHES: [(&str, f64, f64, f64, &str, f64); 28] = [
    ("A", 0.90, 0.95, 27.9, "sprig", 1.6),
    ("A", 0.95, 4.75, 24.9, "bud", 0.7),
    ("A", 0.86, 4.85, 23.1, "flower", 1.0),
    ("A", 0.78, 0.95, 22.4, "umbel", 1.3),
    ("A", 0.70, 0.9, 19.3, "blossoms", 0.75),
    ("A", 0.74, 5.35, 21.4, "blossoms", 0.65),
    ("B", 0.86, 5.55, 16.6, "cyme", 1.9),
    ("A", 0.55, 0.8, 16.0, "bud", 0.7),
    ("C", 0.95, 0.75, 14.6, "cyme", 1.5),
    ("A", 0.60, 5.25, 13.4, "blossoms", 0.8),
    ("C", 0.55, 0.65, 11.4, "bud", 0.6),
    ("B", 0.45, 5.35, 10.6, "umbel", 1.4),
    ("C", 0.62, 0.75, 9.0, "bells", 0.9),
    ("B", 0.28, 5.2, 6.6, "flower", 1.05),
    ("C", 0.30, 0.7, 5.6, "sprig", 2.0),
    ("A", 0.22, 4.7, 3.2, "cyme", 1.6),
    ("B", 0.62, 5.6, 12.0, "tendril", 0.4),
    ("A", 0.40, 1.0, 13.2, "tendril", 0.35),
    ("A", 0.97, 3.9, 28.3, "cyme", 1.2),
    ("A", 0.93, 5.25, 25.4, "blossoms", 0.6),
    ("A", 0.82, 1.0, 24.7, "leaf", 1.4),
    ("B", 0.80, 5.7, 19.9, "bud", 0.6),
    ("B", 0.64, 5.05, 14.9, "leaf", 1.4),
    ("B", 0.38, 5.35, 8.8, "cyme", 1.4),
    ("C", 0.50, 0.75, 7.4, "blossoms", 0.6),
    ("A", 0.10, 0.85, 3.1, "leaf", 1.5),
    ("B", 0.16, 5.6, 4.4, "bud", 0.6),
    ("C", 0.85, 2.6, 11.5, "bud", 0.55),
];

const GHOSTS: [[f64; 3]; 2] = [[3.4, 5.4, 1.5], [2.7, 16.9, 1.25]];
// visitors: kind, x, y (cm), size, heading (degrees); the snail: stem, fraction, side, size
const VISITORS: [(&str, f64, f64, f64, f64); 2] = [("butterfly", 4.3, 20.6, 1.3, 100.0), ("bee", 1.6, 26.6, 0.75, 30.0)];
const SNAIL: (&str, f64, f64, f64) = ("C", 0.35, 1.0, 0.8);
const ACCENTS: [(&str, f64, f64); 5] = [("rosette", 0.45, 4.2), ("sparkle", 0.5, 9.9), ("sparkle", 0.45, 18.3), ("rosette", 0.5, 24.3), ("sparkle", 5.9, 1.6)];

// the footer: the stem runs from the root to this fraction of the text width;
// motifs along it: x (cm), motif, size
const FOOT_END: f64 = 0.45;
const FOOT: [(f64, &str, f64); 4] = [(3.6, "cyme", 1.1), (5.3, "blossoms", 0.5), (7.6, "cyme", 1.0), (9.6, "blossoms", 0.42)];

// ---------------------------------------------------------------- drawing
// the point along a polyline at a fraction of its length, with the unit tangent
fn at(pts: &[Pt], f: f64) -> (f64, f64, f64, f64) {
    let (s, tot) = ink::arclen(pts);
    let want = f * tot;
    for i in 1..pts.len() {
        if s[i] >= want {
            let (tx, ty) = norm(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
            return (pts[i][0], pts[i][1], tx, ty);
        }
    }
    let n = pts.len() - 1;
    let (tx, ty) = norm(pts[n][0] - pts[n - 1][0], pts[n][1] - pts[n - 1][1]);
    (pts[n][0], pts[n][1], tx, ty)
}

fn place(layer: i32, x: f64, y: f64, a: f64, f: impl FnOnce()) {
    ink::layer(layer);
    ink::push();
    ink::translate(x, y);
    ink::rotate(a - 90.0);
    f();
    ink::pop();
}

fn draw_named(name: &str, rng: &Rng, s: f64) {
    match name {
        "sparkle" => flora::sparkle(rng, s, &O::default()),
        "rosette" => flora::rosette(rng, s, &O::default()),
        "butterfly" => flora::butterfly(rng, s, &O::default()),
        "bee" => flora::bee(rng, s, &O::default()),
        _ => {}
    }
}

pub fn page(g: &Geo, seed: f64, visitors: bool) {
    let rng = Rng::new(seed * 7919.0 + 17.0);
    let jit = rng.fork();
    ink::set_transform([ink::CM, 0.0, 0.0, ink::CM, 0.0, 0.0]);

    // stems
    let stems: Vec<Vec<Pt>> = STEMS
        .iter()
        .map(|st| {
            let cp: Vec<Pt> = st
                .pts
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let dx = if i > 0 { jit.sym(0.12) } else { 0.0 };
                    let dy = if i > 0 { jit.sym(0.12) } else { 0.0 };
                    [p[0] + dx, p[1] + dy]
                })
                .collect();
            ink::catmull(&cp, 10)
        })
        .collect();
    let stem_of = |name: &str| STEMS.iter().position(|s| s.name == name).unwrap();
    ink::layer(L_STEM);
    for k in [2, 1, 0] {
        flora::stem(&stems[k], STEMS[k].w[0], STEMS[k].w[1], None, 1.0, None);
    }
    // the scroll at the root
    {
        let pts: Vec<Pt> = (0..=40)
            .map(|i| {
                let t = i as f64 / 40.0;
                let a = PI * 0.5 + t * 1.4 * 2.0 * PI;
                let r = 0.55 * (1.0 - 0.85 * t).powf(1.3);
                [ROOT[0] - 0.55 - r * a.cos(), ROOT[1] + 0.05 + r * a.sin()]
            })
            .collect();
        flora::stem(&pts, 0.06, 0.012, None, 1.0, None);
    }
    // footer stem
    let fx1 = g.textleft + (g.textright - g.textleft) * FOOT_END;
    let mut foot: Vec<Pt> = vec![];
    {
        let f = ink::fbm1(&jit, 2);
        let mut x = ROOT[0];
        while x <= fx1 {
            let u = ((x - ROOT[0]) / 1.5).min(1.0);
            let u = u * u * (3.0 - 2.0 * u);
            foot.push([x, ROOT[1] + 0.1 + (0.95 - ROOT[1] - 0.1 + 0.2 * f.at(x / 3.5)) * u]);
            x += 0.2;
        }
        flora::stem(&foot, 0.065, 0.01, None, 1.0, None);
    }

    // terminal flowers
    for (k, st) in STEMS.iter().enumerate() {
        let pts = &stems[k];
        let (p, q) = (pts[pts.len() - 1], pts[pts.len() - 4]);
        let (ax, ay) = norm(p[0] - q[0], p[1] - q[1]);
        let (kind, s, a) = st.tip;
        let m = grow::motif(kind).unwrap();
        let r = rng.fork();
        place(m.layer, p[0] + ax * 0.2, p[1] + ay * 0.2, a, || (m.draw)(&r, s, None));
    }

    // ghosts behind everything
    for gh in GHOSTS {
        let r = rng.fork();
        place(L_GHOST, gh[0], gh[1], 90.0, || flora::ghost(0.62, || flora::flower(&r, gh[2], &O::default())));
    }

    // leaves at the nodes
    for (st, f, side, len, lobed) in LEAVES {
        let (x, y, tx, ty) = at(&stems[stem_of(st)], f);
        let a = deg(tx, ty) + side * jit.range(40.0, 55.0);
        let l = len * 1.12 * jit.range(0.92, 1.05);
        let r = rng.fork();
        let bend = side * jit.range(0.08, 0.3);
        place(L_LEAF, x, y, a, || {
            ink::scale1(l);
            let depth = if lobed { jit.range(0.45, 0.75) } else { 0.0 };
            flora::leaf(&r, &O { w: Some(0.26), depth: Some(depth), bend: Some(bend), ..O::default() })
        });
    }
    // leaflets along the footer, smaller toward its end
    {
        let (s, tot) = ink::arclen(&foot);
        let (mut side, mut nexts) = (1.0, 0.7);
        for i in 1..foot.len().saturating_sub(2) {
            if s[i] >= nexts {
                let u = s[i] / tot;
                nexts += (0.55 + 0.8 * u) * jit.range(0.85, 1.15);
                side = -side;
                let (tx, ty) = norm(foot[i + 1][0] - foot[i - 1][0], foot[i + 1][1] - foot[i - 1][1]);
                let l = (0.9 - 0.45 * u) * jit.range(0.85, 1.1);
                let a = deg(tx, ty) + side * jit.range(35.0, 55.0);
                let r = rng.fork();
                let bend = -side * 0.2;
                place(L_LEAF, foot[i][0], foot[i][1], a, || {
                    ink::scale1(l);
                    flora::leaflet(&r, &O { bend: Some(bend), ..O::default() })
                });
            }
        }
    }

    // branches and their motifs
    let grow_branch = |sx: f64, sy: f64, tx: f64, ty: f64, px: f64, py: f64, kind: &str, s: f64, w0: f64| {
        let m = grow::motif(kind).unwrap();
        let (dx, dy) = norm(px - sx, py - sy);
        let (ax, ay) = norm(dx, dy + m.upright);
        let (mut ox, mut oy) = (px - ax * (m.off)(s), py - ay * (m.off)(s));
        if m.hang {
            ox = px - dx * 0.2 * s;
            oy = py + 0.45 * s;
        }
        let bpts = flora::branch_path(&jit, sx, sy, tx, ty, ox, oy, if m.hang { dx } else { ax }, if m.hang { -0.3 } else { ay });
        ink::layer(L_STEM);
        flora::stem(&bpts, w0, 0.014, None, 1.0, None);
        let dd = ((ox - sx).powi(2) + (oy - sy).powi(2)).sqrt();
        if dd > 1.0 {
            let i = (bpts.len() as f64 * 0.4).floor() as usize; // 1-based
            let (p, q) = (bpts[i - 1], bpts[i]);
            let a = deg(q[0] - p[0], q[1] - p[1]) + (if jit.chance(0.5) { 1.0 } else { -1.0 }) * jit.range(40.0, 60.0);
            let ls = jit.range(0.45, 0.7);
            let r = rng.fork();
            place(L_LEAF, p[0], p[1], a, || {
                ink::scale1(ls);
                flora::leaflet(&r, &O::default())
            });
        }
        let r = rng.fork();
        place(m.layer, ox, oy, if m.hang { 90.0 } else { deg(ax, ay) }, || (m.draw)(&r, s, None));
    };
    for (st, f, x, y, kind, s) in BRANCHES {
        let (sx, sy, tx, ty) = at(&stems[stem_of(st)], f);
        let (px, py) = (x + jit.sym(0.1), y + jit.sym(0.1));
        grow_branch(sx, sy, tx, ty, px, py, kind, s, 0.035);
    }
    for (x, kind, s) in FOOT {
        if x < fx1 - 0.5 {
            let mut best = f64::INFINITY;
            let mut bi = 0;
            for (i, p) in foot.iter().enumerate() {
                let dd = (p[0] - (x - 0.4)).abs();
                if dd < best {
                    best = dd;
                    bi = i;
                }
            }
            let (p, q) = (foot[bi], foot[(bi + 1).min(foot.len() - 1)]);
            let (tx, ty) = norm(q[0] - p[0], q[1] - p[1]);
            grow_branch(p[0], p[1], tx, ty, x, p[1] + 0.45, kind, s, 0.025);
        }
    }

    // accents and the footer's copper tail
    for (kind, x, y) in ACCENTS {
        let r = rng.fork();
        place(L_ACCENT, x, y, 90.0, || draw_named(kind, &r, if kind == "sparkle" { 0.13 } else { 0.12 }));
    }
    {
        let e = foot[foot.len() - 1];
        let x2 = e[0] + 1.9;
        ink::layer(L_ACCENT);
        ink::moveto(e[0], e[1]);
        ink::curveto(e[0] + 0.6, e[1] + 0.05, x2 - 0.6, e[1] - 0.05, x2, e[1]);
        ink::stroke(ink::accent(0.0), 0.3, 0.8, false);
        place(L_ACCENT, x2 + 0.18, e[1], 90.0, || flora::sparkle(&rng.fork(), 0.12, &O { rays: Some(false), ..O::default() }));
        ink::circle(x2 + 0.45, e[1], 0.025);
        ink::fill1(ink::accent(0.0));
    }

    // visitors last, from their own stream, so the rest of the page does not change
    if visitors {
        let vr = Rng::new(seed * 7919.0 + 103.0);
        for (kind, x, y, s, a) in VISITORS {
            let r = vr.fork();
            place(L_ACCENT, x, y, a, || draw_named(kind, &r, s));
        }
        let (x, y, tx, ty) = at(&stems[stem_of(SNAIL.0)], SNAIL.1);
        let r = vr.fork();
        place(L_ACCENT, x + ty * SNAIL.2 * 0.03, y - tx * SNAIL.2 * 0.03, deg(tx, ty), || {
            ink::scale(SNAIL.2, 1.0);
            flora::snail(&r, SNAIL.3, &O::default())
        });
    }

    // a safety clip: the column (a little past its edge) and the footer band
    ink::layer(0);
    ink::poly(&[[0.0, 0.0], [g.colright + 0.25, 0.0], [g.colright + 0.25, g.ph], [0.0, g.ph]], true);
    let fy = 2.0f64.min(g.footy - 0.2);
    ink::poly(&[[g.colright, 0.0], [fx1 + 3.0, 0.0], [fx1 + 3.0, fy], [g.colright, fy]], true);
    ink::clip();
    ink::layer(99);
    ink::unclip();
}
