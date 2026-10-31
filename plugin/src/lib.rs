// florilegium as a Typst plugin: procedurally generated Delft-blue floral page
// margins, drawn in Rust and handed to Typst as PDF.
//
// Every function takes its arguments as JSON (and the initial a font file too)
// and returns a one-page PDF whose content stream holds the same path
// operators ink.lua writes into a LuaTeX page. Typst embeds such a page as a
// Form XObject and copies the stream as it is, so the output keeps its 0.1 bp
// coordinates and its ExtGState opacity and multiply, where SVG would cost a
// transparency group per blotch and nine digits per number.

pub mod authored;
pub mod flora;
pub mod grow;
pub mod initial;
pub mod ink;
pub mod sumi;

use serde::Deserialize;
use serde_json::Value;
#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::*;

// the plugin protocol only in the WASM build; natively (examples/dump.rs) the
// functions below are plain Rust
#[cfg(target_arch = "wasm32")]
initiate_protocol!();

#[derive(Deserialize)]
struct GeoIn {
    pw: f64,
    ph: f64,
    textleft: f64,
    textright: f64,
    ytop: f64,
    ybot: f64,
    #[serde(default)]
    colright: Option<f64>,
    #[serde(default)]
    headrule: Option<f64>,
    #[serde(default)]
    footy: Option<f64>,
}

impl GeoIn {
    fn geo(&self) -> grow::Geo {
        grow::Geo {
            pw: self.pw,
            ph: self.ph,
            textleft: self.textleft,
            textright: self.textright,
            ytop: self.ytop,
            ybot: self.ybot,
            colright: self.colright.unwrap_or(self.textleft - 0.4),
            headrule: self.headrule.unwrap_or(self.ytop + 0.7),
            footy: self.footy.unwrap_or(self.ybot - 1.05),
        }
    }
}

#[derive(Deserialize)]
struct MarkIn {
    #[serde(default)]
    heading: Option<i64>,
    #[serde(default)]
    motif: String,
    y: f64,
}

#[derive(Deserialize)]
struct FrameIn {
    k: usize,
    n: usize,
    #[serde(default)]
    explain: bool,
}

#[derive(Deserialize)]
struct PageIn {
    #[serde(default = "grown")]
    mode: String,
    geo: GeoIn,
    page: f64,
    #[serde(default)]
    seed: Option<f64>,
    #[serde(default)]
    spill: bool,
    #[serde(default)]
    spilly: f64,
    #[serde(default = "yes")]
    visitors: bool,
    #[serde(default)]
    age: Option<f64>,
    #[serde(default)]
    marks: Vec<MarkIn>,
    #[serde(default)]
    ink: Option<[f64; 3]>,
    #[serde(default)]
    accent: Option<[f64; 3]>,
    #[serde(default)]
    bloom: Option<[f64; 3]>,
    #[serde(default)]
    frame: Option<FrameIn>,
    #[serde(default = "yes")]
    head: bool,
}

fn grown() -> String {
    "grown".into()
}
fn yes() -> bool {
    true
}

// a fresh page in the given colors
fn begin(inkc: Option<[f64; 3]>, accent: Option<[f64; 3]>, bloom: Option<[f64; 3]>) {
    ink::reset();
    ink::set_ink(inkc.unwrap_or([0x24 as f64 / 255.0, 0x56 as f64 / 255.0, 0xC4 as f64 / 255.0]));
    ink::set_accent(accent.unwrap_or([0xB4 as f64 / 255.0, 0x66 as f64 / 255.0, 0x2F as f64 / 255.0]));
    ink::set_bloom(bloom);
}

// one seed per page: from the page number, or from a fixed seed and the page number
fn page_seed(page: f64, fixed: Option<f64>) -> f64 {
    match fixed {
        Some(f) => f * 7919.0 + page * 131.0,
        None => page * 97.0 + 1030.0,
    }
}

// a one-page PDF of the region [x0, x0+w] x [y0, y0+h] (bp, y up) around `content`
fn pdf(content: &str, x0: f64, y0: f64, w: f64, h: f64) -> Vec<u8> {
    pdf_with(content, &ink::resources(), x0, y0, w, h)
}
fn pdf_with(content: &str, resources: &str, x0: f64, y0: f64, w: f64, h: f64) -> Vec<u8> {
    let shift = if x0 != 0.0 || y0 != 0.0 { format!("1 0 0 1 {:.2} {:.2} cm\n", -x0, -y0) } else { String::new() };
    // uncompressed: Typst compresses the page it embeds anyway, and deflate
    // would be most of the plugin's time under Typst's WASM interpreter
    let stream = format!("{shift}{content}").into_bytes();
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w:.2} {h:.2}] /Resources << /ExtGState << {resources} >> >> /Contents 4 0 R >>"
        )
        .into_bytes(),
        [format!("<< /Length {} >>\nstream\n", stream.len()).as_bytes(), &stream, b"\nendstream"].concat(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

// the header: a copper ivy leaf whose stem becomes the head rule
fn head(g: &grow::Geo, rng: &ink::Rng) {
    ink::set_transform([ink::CM, 0.0, 0.0, ink::CM, 0.0, 0.0]);
    ink::layer(100);
    ink::push();
    ink::translate(g.colright - 0.55, g.headrule + 0.02);
    let (x, y) = flora::hedera(&rng.fork(), 0.36, &flora::O::default());
    ink::pop();
    let e = g.textright;
    ink::moveto(x, y);
    ink::curveto(x + 1.0, y, x + 1.5, g.headrule, x + 2.5, g.headrule);
    ink::lineto(e, g.headrule);
    ink::stroke1(ink::accent(0.0), 0.4);
    ink::circle(e, g.headrule, 0.03);
    ink::fill1(ink::accent(0.0));
}

fn grow_input(a: &PageIn, g: &grow::Geo, seed: f64, anim: bool) -> grow::Input {
    grow::Input {
        geo: g.clone(),
        seed,
        spill: a.spill,
        spilly: a.spilly,
        visitors: a.visitors,
        age: a.age,
        marks: a.marks.iter().map(|m| grow::Mark { heading: m.heading, motif: m.motif.clone(), y: m.y }).collect(),
        anim,
    }
}

// The margin of one page, the size of the paper. Grown, or authored with
// "mode": "authored"; with "frame": {k, n, explain} frame k of n of the
// garden's growth instead.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn page(arg: &[u8]) -> Result<Vec<u8>, String> {
    let a: PageIn = serde_json::from_slice(arg).map_err(|e| format!("florilegium: bad page arguments: {e}"))?;
    if let Some(fr) = &a.frame {
        return Ok(frame_of(&grown_recording(&a), fr));
    }
    begin(a.ink, a.accent, a.bloom);
    let g = a.geo.geo();
    let seed = page_seed(a.page, a.seed);
    if a.mode == "authored" {
        authored::page(&g, seed, a.visitors);
    } else if a.mode == "plum" {
        let gi = grow_input(&a, &g, seed, false);
        sumi::page(&sumi::Input { geo: gi.geo, seed, spill: gi.spill, spilly: gi.spilly, age: gi.age, marks: gi.marks });
    } else {
        grow::page(&grow_input(&a, &g, seed, false));
    }
    if a.head && a.mode == "plum" {
        sumi::head(&g, &ink::Rng::new(seed));
    } else if a.head {
        head(&g, &ink::Rng::new(seed));
    }
    Ok(pdf(&ink::flush(), 0.0, 0.0, g.pw * ink::CM, g.ph * ink::CM))
}

// ---------------------------------------------------------------- the growth animation
// A garden recorded once, with its events timed: every frame writes the pieces
// whose time has come, then the header, which does not grow.
struct Recorded {
    rec: ink::Recording,
    events: Vec<grow::Event>,
    total: f64,
    head: String,
    resources: String,
    size: (f64, f64),
}

fn grown_recording(a: &PageIn) -> Recorded {
    begin(a.ink, a.accent, a.bloom);
    let g = a.geo.geo();
    let seed = page_seed(a.page, a.seed);
    let (rec, events, total) = grow::page(&grow_input(a, &g, seed, true)).rec.expect("florilegium: no recording");
    if a.head {
        head(&g, &ink::Rng::new(seed));
    }
    let head = ink::flush();
    Recorded { rec, events, total, head, resources: ink::resources(), size: (g.pw * ink::CM, g.ph * ink::CM) }
}

fn frame_of(r: &Recorded, fr: &FrameIn) -> Vec<u8> {
    // the last frame is the page: every event finished (total itself may round below)
    let t = if fr.k >= fr.n { f64::INFINITY } else { r.total * (fr.k as f64 - 1.0) / (fr.n.max(2) - 1) as f64 };
    let mut body = ink::emit(&r.rec, |tag| {
        if tag.overlay && !fr.explain {
            return false;
        }
        let e = &r.events[tag.ev];
        tag.visible((t - e.t0.unwrap_or(0.0)) / e.dur)
    });
    body.push_str(&r.head);
    pdf_with(&body, &r.resources, 0.0, 0.0, r.size.0, r.size.1)
}

thread_local! {
    static RECORDED: std::cell::RefCell<Option<Recorded>> = const { std::cell::RefCell::new(None) };
}

// Record the growth of a page's garden (the arguments of page(), without the
// frame) and keep it in the plugin's memory. Typst calls this through
// plugin.transition(), which returns a plugin holding the recording, once per
// garden; frame() on that plugin then only writes.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn record(arg: &[u8]) -> Result<Vec<u8>, String> {
    let a: PageIn = serde_json::from_slice(arg).map_err(|e| format!("florilegium: bad record arguments: {e}"))?;
    let r = grown_recording(&a);
    RECORDED.with(|c| *c.borrow_mut() = Some(r));
    Ok(vec![])
}

// Frame {k, n, explain} of the garden record() kept.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn frame(arg: &[u8]) -> Result<Vec<u8>, String> {
    let fr: FrameIn = serde_json::from_slice(arg).map_err(|e| format!("florilegium: bad frame arguments: {e}"))?;
    RECORDED.with(|c| c.borrow().as_ref().map(|r| frame_of(r, &fr)))
        .ok_or_else(|| "florilegium: frame() before record()".to_string())
}

#[derive(Deserialize)]
struct InitialIn {
    letter: String,
    side: f64, // cm
    #[serde(default)]
    seed: f64,
    #[serde(default)]
    tone: Option<f64>,
    #[serde(default)]
    ink: Option<[f64; 3]>,
    #[serde(default)]
    accent: Option<[f64; 3]>,
    #[serde(default)]
    bloom: Option<[f64; 3]>,
}

const TILE_PAD: f64 = 0.6;

// A decorated initial: a tile of side `side` cm (plus TILE_PAD bp all round),
// the letter's outline read from `font`.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn initial(arg: &[u8], font: &[u8]) -> Result<Vec<u8>, String> {
    let a: InitialIn = serde_json::from_slice(arg).map_err(|e| format!("florilegium: bad initial arguments: {e}"))?;
    begin(a.ink, a.accent, a.bloom);
    let ch = a.letter.chars().next().ok_or("florilegium: an initial needs a letter")?;
    let rng = ink::Rng::new(a.seed * 7919.0 + 29.0);
    initial::draw(&rng, a.side, ch, font, a.tone).map_err(|e| format!("florilegium: {e}"))?;
    // a margin of TILE_PAD bp all round: the frame's stroke is centered on the edge
    let s = a.side * ink::CM;
    Ok(pdf(&ink::flush(), -TILE_PAD, -TILE_PAD, s + 2.0 * TILE_PAD, s + 2.0 * TILE_PAD))
}

#[derive(Deserialize)]
struct DrawIn {
    name: String,
    #[serde(default = "one")]
    size: f64,
    #[serde(default)]
    opts: Value,
    #[serde(default)]
    seed: f64,
    #[serde(default)]
    ghost: Option<f64>,
    #[serde(default)]
    angle: f64,
    #[serde(default)]
    ink: Option<[f64; 3]>,
    #[serde(default)]
    accent: Option<[f64; 3]>,
    #[serde(default)]
    bloom: Option<[f64; 3]>,
}
fn one() -> f64 {
    1.0
}

fn color(v: &Value) -> Option<ink::Color> {
    let a = v.as_array()?;
    Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?])
}

fn opts(v: &Value) -> flora::O {
    let f = |k: &str| v.get(k).and_then(Value::as_f64);
    let i = |k: &str| v.get(k).and_then(Value::as_i64);
    let b = |k: &str| v.get(k).and_then(Value::as_bool);
    flora::O {
        w: f("w"),
        bend: f("bend"),
        tone: f("tone"),
        streaks: i("streaks"),
        pointed: v.get("shape").and_then(Value::as_str) == Some("pointed"),
        heart: b("heart"),
        petals: i("petals"),
        tilt: f("tilt"),
        eye: v.get("eye").and_then(color),
        star: b("star"),
        side: f("side"),
        fork: b("fork"),
        depth: f("depth"),
        lobes: i("lobes"),
        open: f("open"),
        trail: b("trail"),
        facing: f("facing"),
        drift: i("drift"),
        color: v.get("color").and_then(color),
        rays: b("rays"),
    }
}

// One primitive at the origin, turned by `angle` degrees: its operators and
// their extent in bp around the origin (padded for the line widths).
fn primitive(arg: &[u8]) -> Result<(String, [f64; 4]), String> {
    let a: DrawIn = serde_json::from_slice(arg).map_err(|e| format!("florilegium: bad draw arguments: {e}"))?;
    begin(a.ink, a.accent, a.bloom);
    let rng = ink::Rng::new(a.seed * 7919.0 + 31.0);
    let o = opts(&a.opts);
    let old = ink::set_lift(a.ghost.unwrap_or(0.0));
    ink::rotate(a.angle);
    let known = match a.name.as_str() {
        // the sprays of grow's motif table, by their motif names
        "blossoms" | "bells" | "budding" => {
            (grow::motif(&a.name).unwrap().draw)(&rng, a.size, None);
            true
        }
        name => flora::draw(name, &rng, a.size, &o),
    };
    ink::set_lift(old);
    if !known {
        return Err(format!("florilegium: unknown primitive '{}'", a.name));
    }
    let body = ink::flush();
    let b = ink::bbox();
    if !b[0].is_finite() {
        return Ok((body, [0.0, 0.0, 1.0, 1.0]));
    }
    let pad = 2.0;
    Ok((body, [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]))
}

#[derive(Deserialize)]
struct SprayIn {
    w: f64,
    h: f64,
    #[serde(default = "one")]
    seed: f64,
    #[serde(default)]
    age: Option<f64>,
    #[serde(default)]
    flip: bool,
    #[serde(default)]
    ink: Option<[f64; 3]>,
    #[serde(default)]
    accent: Option<[f64; 3]>,
    #[serde(default)]
    bloom: Option<[f64; 3]>,
}

// A plum spray in a band of w x h cm: {w, h, seed, age, flip}, a PDF of that size.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn spray(arg: &[u8]) -> Result<Vec<u8>, String> {
    let a: SprayIn = serde_json::from_slice(arg).map_err(|e| format!("florilegium: bad spray arguments: {e}"))?;
    begin(a.ink, a.accent, a.bloom);
    sumi::spray(a.w, a.h, a.seed, a.age.unwrap_or(0.5), a.flip);
    Ok(pdf(&ink::flush(), 0.0, 0.0, a.w * ink::CM, a.h * ink::CM))
}

// One primitive on its own, cropped to what it draws: {name, size, opts, angle}.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn draw(arg: &[u8]) -> Result<Vec<u8>, String> {
    let (body, b) = primitive(arg)?;
    Ok(pdf(&body, b[0], b[1], b[2] - b[0], b[3] - b[1]))
}

// Where draw() crops: [x0, y0, x1, y1] in bp around the primitive's origin, y up.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn extent(arg: &[u8]) -> Result<Vec<u8>, String> {
    let (_, b) = primitive(arg)?;
    Ok(format!("[{:.2}, {:.2}, {:.2}, {:.2}]", b[0], b[1], b[2], b[3]).into_bytes())
}
