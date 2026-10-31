// initial.rs -- a decorated initial for florilegium (a port of initial.lua).
//
// A square tile at the start of a paragraph: the letter, its outline read from
// an OpenType font and painted like a petal; a vine that winds over and under
// the letter's strokes, as in an interlaced manuscript initial; leaves where
// they fit and small flowers in the largest gaps; a double frame, like a tile.
// Drawn in a local frame of centimeters, [0,S] x [0,S], under the current ink
// transform.

use crate::flora::{self, O};
use crate::ink::{self, lerp, tone, Pt, Rng, CM, RAD, WHITE};

fn dist(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

// ---------------------------------------------------------------- outlines
enum Op {
    M(f64, f64),
    L(f64, f64),
    C(f64, f64, f64, f64, f64, f64),
}

// one contour, as path commands and as a flattened polygon
struct Contour {
    ops: Vec<Op>,
    poly: Vec<Pt>,
}

// collects a glyph's outline in font units; quadratics become cubics
#[derive(Default)]
struct Outline {
    cs: Vec<Contour>,
    last: Pt,
}

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = (x as f64, y as f64);
        self.cs.push(Contour { ops: vec![Op::M(x, y)], poly: vec![[x, y]] });
        self.last = [x, y];
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = (x as f64, y as f64);
        if let Some(c) = self.cs.last_mut() {
            c.ops.push(Op::L(x, y));
            c.poly.push([x, y]);
        }
        self.last = [x, y];
    }
    fn quad_to(&mut self, qx: f32, qy: f32, x: f32, y: f32) {
        let [lx, ly] = self.last;
        let (qx, qy, x, y) = (qx as f64, qy as f64, x as f64, y as f64);
        let (x1, y1) = (lx + 2.0 / 3.0 * (qx - lx), ly + 2.0 / 3.0 * (qy - ly));
        let (x2, y2) = (x + 2.0 / 3.0 * (qx - x), y + 2.0 / 3.0 * (qy - y));
        self.curve_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let [lx, ly] = self.last;
        let (x1, y1, x2, y2, x, y) = (x1 as f64, y1 as f64, x2 as f64, y2 as f64, x as f64, y as f64);
        if let Some(c) = self.cs.last_mut() {
            c.ops.push(Op::C(x1, y1, x2, y2, x, y));
            let f3 = ink::bezier3(lx, ly, x1, y1, x2, y2, x, y);
            for i in 1..=8 {
                let (px, py) = f3(i as f64 / 8.0);
                c.poly.push([px, py]);
            }
        }
        self.last = [x, y];
    }
    fn close(&mut self) {}
}

// the contours of one character, scaled to fit a box of height h and width w
// (cm) centered at (cx, cy)
fn contours(font: &[u8], ch: char, cx: f64, cy: f64, h: f64, w: f64) -> Result<Vec<Contour>, String> {
    let face = ttf_parser::Face::parse(font, 0).map_err(|e| format!("cannot read the initial's font: {e}"))?;
    let gid = face.glyph_index(ch).ok_or_else(|| format!("no glyph for '{ch}' in the initial's font"))?;
    let mut o = Outline::default();
    let bb = face.outline_glyph(gid, &mut o).ok_or_else(|| format!("no outline for '{ch}' in the initial's font"))?;
    let (x0, y0, x1, y1) = (bb.x_min as f64, bb.y_min as f64, bb.x_max as f64, bb.y_max as f64);
    let k = (h / (y1 - y0)).min(w / (x1 - x0));
    let (ox, oy) = (cx - k * (x0 + x1) / 2.0, cy - k * (y0 + y1) / 2.0);
    let p = |x: f64, y: f64| (ox + k * x, oy + k * y);
    Ok(o.cs
        .into_iter()
        .map(|c| Contour {
            ops: c
                .ops
                .into_iter()
                .map(|op| match op {
                    Op::M(x, y) => {
                        let (x, y) = p(x, y);
                        Op::M(x, y)
                    }
                    Op::L(x, y) => {
                        let (x, y) = p(x, y);
                        Op::L(x, y)
                    }
                    Op::C(a, b, c, d, e, f) => {
                        let ((a, b), (c, d), (e, f)) = (p(a, b), p(c, d), p(e, f));
                        Op::C(a, b, c, d, e, f)
                    }
                })
                .collect(),
            poly: c.poly.into_iter().map(|q| {
                let (x, y) = p(q[0], q[1]);
                [x, y]
            }).collect(),
        })
        .collect())
}

fn trace(cs: &[Contour]) {
    for c in cs {
        for op in &c.ops {
            match *op {
                Op::M(x, y) => ink::moveto(x, y),
                Op::L(x, y) => ink::lineto(x, y),
                Op::C(a, b, c, d, e, f) => ink::curveto(a, b, c, d, e, f),
            }
        }
        ink::close();
    }
}

// even-odd point in the letter
fn inside(cs: &[Contour], x: f64, y: f64) -> bool {
    let mut c = false;
    for ct in cs {
        let p = &ct.poly;
        let n = p.len();
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi, xj, yj) = (p[i][0], p[i][1], p[j][0], p[j][1]);
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                c = !c;
            }
            j = i;
        }
    }
    c
}

// ---------------------------------------------------------------- the tile
// S: side (cm); letter: one character; font: an OpenType or TrueType file
pub fn draw(rng: &Rng, s: f64, letter: char, font: &[u8], t0: Option<f64>) -> Result<(), String> {
    let m = 0.07 * s; // the frame's inset
    let cs = contours(font, letter, s * 0.5, s * 0.47, s * 0.66, s * 0.7)?;
    // free space: a grid of the distance to the letter and the frame
    const N: usize = 36;
    let cell = s / N as f64;
    let mut blocked = vec![];
    let mut freecells = vec![];
    for i in 0..N {
        for j in 0..N {
            let (x, y) = ((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
            if inside(&cs, x, y) { blocked.push([x, y]) } else { freecells.push([x, y]) }
        }
    }
    let clearance = |x: f64, y: f64| {
        let mut dd = (x - m).min(s - m - x).min(y - m).min(s - m - y);
        for b in &blocked {
            let e = dist(x, y, b[0], b[1]) - cell * 0.7;
            if e < dd {
                dd = e;
            }
        }
        dd
    };

    // the ground: a pale glaze and a double frame
    ink::layer(1);
    let sq = [[0.0, 0.0], [s, 0.0], [s, s], [0.0, s]];
    ink::poly(&sq, true);
    ink::clip();
    ink::flood(tone(0.975), 0.0, 0.0, s, s);
    flora::mottle(rng, 0.0, 0.0, s, s, 6, s * 0.2, 0.93, 1.0, 0);
    ink::unclip();
    ink::poly(&sq, true);
    ink::stroke1(tone(0.1), 0.8);
    let h = m * 0.5;
    ink::poly(&[[h, h], [s - h, h], [s - h, s - h], [h, s - h]], true);
    ink::stroke(tone(0.15), 0.3, 0.8, false);

    // the vine: from the lower-left corner up across the letter to the upper right
    let f = ink::fbm1(rng, 2);
    let mut ctrl = vec![[m * 1.2, m * 1.2]];
    for (x0, x1, y0, y1) in [(0.14, 0.22, 0.3, 0.4), (0.32, 0.42, 0.5, 0.58), (0.58, 0.68, 0.42, 0.52), (0.74, 0.82, 0.62, 0.72)] {
        let x = s * rng.range(x0, x1);
        let y = s * rng.range(y0, y1);
        ctrl.push([x, y]);
    }
    ctrl.push([s - m * 1.6, s - m * 1.4]);
    let mut pts = ink::catmull(&ctrl, 14);
    let np = pts.len();
    for (i, p) in pts.iter_mut().enumerate() {
        let u = i as f64 / (np - 1) as f64;
        p[1] += 0.03 * s * f.at(u * 6.0);
    }
    let (w0, w1) = (0.026 * s, 0.01 * s);
    let width = |i: usize| lerp(w0, w1, i as f64 / (np - 1) as f64);
    ink::layer(2);
    flora::stem(&pts, w0, w1, None, 1.0, None);
    // where it crosses the letter, it goes over and under in turn
    let mut runs: Vec<[usize; 2]> = vec![];
    let mut inrun = false;
    for (i, p) in pts.iter().enumerate() {
        if inside(&cs, p[0], p[1]) {
            if inrun {
                runs.last_mut().unwrap()[1] = i;
            } else {
                runs.push([i, i]);
                inrun = true;
            }
        } else {
            inrun = false;
        }
    }
    // the strands that pass over: drawn again on top of the letter, inside its
    // outline only, with a thin halo of paper that parts them from the letter
    let mut over = rng.chance(0.5);
    for r in &runs {
        if over {
            let (a, b) = (r[0].saturating_sub(2), (r[1] + 2).min(np - 1));
            let seg = &pts[a..=b];
            ink::layer(4);
            trace(&cs);
            ink::clip();
            ink::smooth(seg, false);
            ink::stroke1(ink::PAPER, (width(a) + width(b)) / 2.0 * ink::unit() * CM + 1.0);
            flora::stem(seg, width(a), width(b), None, 1.0, None);
            ink::unclip();
        }
        over = !over;
    }
    // a bud at its tip
    {
        let (e, q) = (pts[np - 1], pts[np - 4]);
        ink::layer(5);
        ink::push();
        ink::translate(e[0], e[1]);
        ink::rotate((e[1] - q[1]).atan2(e[0] - q[0]) / RAD - 90.0);
        flora::bud(rng, 0.13 * s, &O::default());
        ink::pop();
    }

    // the letter: wash, granulation, pooled edges, light lifted out
    let t0 = t0.unwrap_or_else(|| rng.range(0.16, 0.22));
    ink::layer(3);
    trace(&cs);
    ink::fill1(tone(t0));
    trace(&cs);
    ink::clip();
    flora::mottle(rng, 0.0, 0.0, s, s, 22, s * 0.06, t0 - 0.12, t0 + 0.2, 20);
    for k in 1..=9 {
        let x = s * (0.15 + 0.7 * (k as f64 - 0.5) / 9.0) + rng.sym(0.02 * s);
        let (y0, y1) = (s * rng.range(0.1, 0.25), s * rng.range(0.72, 0.9));
        let x3 = x + rng.sym(0.03 * s);
        let op = rng.range(0.25, 0.45);
        flora::streak3(x, y0, x + 0.02 * s, lerp(y0, y1, 0.35), x - 0.02 * s, lerp(y0, y1, 0.7), x3, y1, 0.018 * s, 0.003 * s, WHITE, op, false);
    }
    trace(&cs);
    ink::stroke(tone(0.04), 0.07 * s * ink::unit() * CM, 0.45, true);
    ink::unclip();
    trace(&cs);
    ink::stroke(tone(0.04), 0.35, 0.7, false);

    // leaves along the vine where they fit, alternating sides
    ink::layer(4);
    let mut sum = 0.0;
    let mut acc = s * rng.range(0.08, 0.14);
    let mut side = 1.0;
    for i in 1..np.saturating_sub(3) {
        sum += dist(pts[i][0], pts[i][1], pts[i - 1][0], pts[i - 1][1]);
        if sum >= acc {
            acc += s * rng.range(0.11, 0.16);
            side = -side;
            let (tx, ty) = (pts[i + 1][0] - pts[i - 1][0], pts[i + 1][1] - pts[i - 1][1]);
            let a = ty.atan2(tx) / RAD + side * rng.range(40.0, 60.0);
            let l = s * rng.range(0.16, 0.22);
            let mut ok = true;
            for u in [0.35, 0.65, 0.95] {
                let (x, y) = (pts[i][0] + l * u * (a * RAD).cos(), pts[i][1] + l * u * (a * RAD).sin());
                if inside(&cs, x, y) || clearance(x, y) < 0.03 * s {
                    ok = false;
                }
            }
            if ok {
                ink::push();
                ink::translate(pts[i][0], pts[i][1]);
                ink::rotate(a - 90.0);
                ink::scale1(l);
                flora::leaflet(rng, &O { bend: Some(-side * 0.2), ..O::default() });
                ink::pop();
            }
        }
    }

    // flowers in the largest gaps: a blossom, and florets on hairlines from the vine
    ink::layer(5);
    let mut spots: Vec<[f64; 3]> = freecells
        .iter()
        .filter_map(|c| {
            let cl = clearance(c[0], c[1]);
            (cl > 0.06 * s).then_some([c[0], c[1], cl])
        })
        .collect();
    spots.sort_by(|a, b| b[2].partial_cmp(&a[2]).unwrap_or(std::cmp::Ordering::Equal));
    let mut placed: Vec<[f64; 3]> = vec![];
    let mut nb = 0;
    for sp in &spots {
        if nb >= 4 {
            break;
        }
        let far = placed.iter().all(|p| dist(sp[0], sp[1], p[0], p[1]) >= p[2] + sp[2] + 0.04 * s);
        if far {
            nb += 1;
            let r = (sp[2] * 0.85).min(0.12 * s);
            placed.push([sp[0], sp[1], r]);
            // a hairline stalk back to the vine, if the vine is near
            let mut best = f64::INFINITY;
            let mut bp = pts[0];
            for p in &pts {
                let dd = dist(p[0], p[1], sp[0], sp[1]);
                if dd < best {
                    best = dd;
                    bp = *p;
                }
            }
            if best < 0.3 * s {
                ink::layer(2);
                ink::moveto(bp[0], bp[1]);
                ink::curveto(lerp(bp[0], sp[0], 0.3), lerp(bp[1], sp[1], 0.1) + 0.03 * s, lerp(bp[0], sp[0], 0.7), sp[1] - 0.02 * s, sp[0], sp[1]);
                ink::stroke1(tone(0.12), 0.35);
                ink::layer(5);
            }
            ink::push();
            ink::translate(sp[0], sp[1]);
            if r > 0.07 * s && nb <= 2 {
                flora::blossom(rng, r, &O::default());
            } else {
                let star = rng.chance(0.5);
                flora::floret(rng, r * 0.8, &O { star: Some(star), ..O::default() });
            }
            ink::pop();
        }
    }
    // a copper sparkle in a corner
    ink::layer(6);
    ink::push();
    ink::translate(s - m * 2.2, m * 2.2);
    flora::sparkle(rng, 0.045 * s, &O::default());
    ink::pop();
    Ok(())
}
