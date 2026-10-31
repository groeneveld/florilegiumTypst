// florilegium.typ -- a procedurally generated floral page margin, drawn by a
// WASM plugin (plugin/, Rust) and placed by Typst as an embedded PDF page.
//
// The geometry of every petal, leaf and stem is computed in the plugin; Typst
// tells it where the page's text area is, where the headings and marks stand,
// where the text ends and how old the garden is, and places the drawing it
// returns behind the page.
//
// Usage
//   #import "florilegium.typ": *
//   #show: florilegium.with(composition: "grown")   // or "authored" (hand-placed, A4)
//
//   options of florilegium(..):
//     composition  "grown" (computed to fit the page), "authored" (the hand-placed
//                  score, for A4 with a 68 mm margin) or "plum" (a sumi-e plum branch);
//                  auto: the style's
//     seed         none: a seed per page from its number; an integer: a fixed seed
//     style        "delft" (cobalt on white, copper hairlines, the grown garden) or
//                  "sumi-e" (a plum branch in black ink with vermilion blossoms, on
//                  warm paper)
//     ink, accent  the hue of the whole palette, and the copper of the hairlines
//                  (auto: the style's)
//     blossom      the hue of the flowers' washes, none: the ink (auto: the style's)
//     fill         the paper (auto: the style's; white for delft)
//     visitors     false leaves out the butterflies, bees, snails and birds
//     sections     true: a flower beside every heading of level 1 and 2 (grown)
//     aging        true: spring on the first page, autumn on the last (grown)
//     margin       false: no margin at all, only initials and draw()
//     paper, page-margin, headsep, footskip, header, footer: the page
//
//   #bloom()  or  #bloom("lily")   (grown) a flower, or any motif, in the margin
//                                  beside the current line
//   #age(0.8)                      (grown) the age of the following pages: 0 spring,
//                                  0.5 summer, 1 autumn (without the option aging)
//   #textend()                     at the very end of the text (grown): the garden
//                                  then also fills the space below it
//   #initial("E", lines: 5)[very page ...]
//                                  a decorated initial, the paragraph wrapped beside it
//   #frame(k, n, explain: false)   (grown) the following pages show frame k of n of
//                                  their garden's growth (animation.typ)
//   #headpiece(15cm, 3cm, seed: 2)  a sumi-e plum spray filling a band, as an image
//   #draw("flower", size: 1.2, opts: (:))
//                                  one primitive on its own, as an image

#let _plugin = plugin("florilegium.wasm")
#let _styles = (
  "delft": (composition: "grown", ink: rgb("#2456C4"), accent: rgb("#B4662F"), blossom: none, fill: auto),
  "sumi-e": (composition: "plum", ink: rgb("#1E1C1A"), accent: rgb("#B8322A"), blossom: rgb("#C8302C"), fill: rgb("#F6F1E6")),
)
#let _config = state("florilegium-config", (seed: none) + _styles.delft)

#let _rgb(c) = c.rgb().components(alpha: false).map(x => x / 100%)
// the colors of a configuration, for the plugin
#let _colors(cfg) = (
  ink: _rgb(cfg.ink), accent: _rgb(cfg.accent),
  bloom: if cfg.blossom != none { _rgb(cfg.blossom) },
)
#let _cm(l) = l / 1cm
#let _img(data, ..args) = image(data, format: "pdf", ..args)

// ---------------------------------------------------------------- marks in the text
// The margin follows the text: bloom() leaves a mark in the line, and the
// background of the page finds it (and, with the option sections, the
// headings) by querying the document, so the garden knows the height of those
// lines on the same page.
#let bloom(..args) = {
  let motif = args.pos().at(0, default: "")
  [#metadata(motif)<florilegium-bloom>]
}
#let age(a) = [#metadata(a)<florilegium-age>]
#let textend() = [#metadata("end")<florilegium-end>]
#let frame(k, n, explain: false) = [#metadata((k: k, n: n, explain: explain))<florilegium-frame>]

// the latest of the marks `label` on or before page `pg`, or none
#let _latest(label, pg) = {
  let ms = query(label).filter(m => m.location().page() <= pg)
  if ms.len() > 0 { ms.last().value } else { none }
}

// ---------------------------------------------------------------- the page
#let _background(cfg) = context {
  // Typst lays the document out at least twice when it asks where things are;
  // in the first pass nothing has a position yet, so the garden would be grown
  // for a page without marks or text end and then grown again. The sentinel
  // at the end of the document is found from the second pass on.
  if query(<florilegium-done>).len() == 0 { return }
  let pg = here().page()
  let number = counter(page).get().first()
  let (pw, ph) = (_cm(page.width), _cm(page.height))
  let m = cfg.page-margin
  let geo = (
    pw: pw, ph: ph,
    textleft: _cm(m.left), textright: pw - _cm(m.right),
    ytop: ph - _cm(m.top), ybot: _cm(m.bottom),
    colright: _cm(m.left) - 0.4,
    headrule: ph - _cm(m.top) + _cm(cfg.headsep) - 0.1,
    footy: _cm(m.bottom) - _cm(cfg.footskip),
  )
  let args = (
    mode: cfg.composition, geo: geo, page: number, seed: cfg.seed,
    visitors: cfg.visitors,
  ) + _colors(cfg)
  if cfg.composition in ("grown", "plum") {
    // marks: headings (option sections) and bloom(), in document order
    let sel = if cfg.sections { selector(heading).or(<florilegium-bloom>) } else { selector(<florilegium-bloom>) }
    let marks = ()
    for el in query(sel) {
      let loc = el.location()
      if loc.page() != pg { continue }
      let y = _cm(loc.position().y)
      if el.func() == heading {
        // the position of a heading is the top of its first line; its baseline
        // sits about a cap height below
        if el.level <= 2 { marks.push((heading: el.level, y: ph - y - 0.25)) }
      } else {
        marks.push((motif: el.value, y: ph - y))
      }
    }
    args.marks = marks
    // the end of the text, on its page
    let ends = query(<florilegium-end>).filter(e => e.location().page() == pg)
    if ends.len() > 0 {
      args.spill = true
      args.spilly = ph - _cm(ends.first().location().position().y)
    }
    // the season: spread over the document, or set by age()
    if cfg.aging {
      let last = counter(page).final().first()
      args.age = if last > 1 { calc.min(1, (number - 1) / (last - 1)) } else { 0.5 }
    } else {
      args.age = _latest(<florilegium-age>, pg)
    }
    // a frame of the growth animation: the garden is recorded once, by a
    // transition of the plugin that keeps it in memory, and every frame is
    // written from that recording
    let fr = _latest(<florilegium-frame>, pg)
    if fr != none and cfg.composition == "grown" {
      let rec = plugin.transition(_plugin.record, bytes(json.encode(args, pretty: false)))
      return _img(rec.frame(bytes(json.encode(fr, pretty: false))), width: page.width, height: page.height)
    }
  }
  _img(_plugin.page(bytes(json.encode(args, pretty: false))), width: page.width, height: page.height)
}

#let florilegium(
  composition: auto,
  seed: none,
  style: "delft",
  ink: auto,
  accent: auto,
  blossom: auto,
  fill: auto,
  visitors: true,
  sections: false,
  aging: false,
  margin: true,
  paper: "a4",
  page-margin: (left: 68mm, right: 22mm, top: 32mm, bottom: 30mm),
  headsep: 8mm,
  footskip: 30pt,
  header: none,
  footer: auto,
  body,
) = {
  assert(style in _styles, message: "florilegium: style is \"delft\" or \"sumi-e\"")
  let pick(v, k) = if v == auto { _styles.at(style).at(k) } else { v }
  let composition = pick(composition, "composition")
  assert(composition in ("grown", "authored", "plum"), message: "florilegium: composition is \"grown\", \"authored\" or \"plum\"")
  let cfg = (
    composition: composition, seed: seed, visitors: visitors,
    ink: pick(ink, "ink"), accent: pick(accent, "accent"), blossom: pick(blossom, "blossom"),
    sections: sections, aging: aging, page-margin: page-margin, headsep: headsep, footskip: footskip,
  )
  set page(
    paper: paper,
    fill: pick(fill, "fill"),
    margin: page-margin,
    header: header,
    header-ascent: headsep,
    footer: if footer == auto { context align(right, text(size: 0.9em, counter(page).display())) } else { footer },
    footer-descent: footskip - 1em,
    background: if margin { _background(cfg) },
  )
  _config.update(cfg)
  body
  [#metadata(none)<florilegium-done>]
}

// ---------------------------------------------------------------- initials
// A decorated initial: a square tile from the cap height of the first line to
// the baseline of line `lines`, the paragraph's first `lines` lines set beside
// it. The letter is also set in the text, invisibly, so that the paragraph
// still copies and searches as "Every", not "very".

// the paragraph as a list of pieces it may break between: words and spaces
// of its text, anything else whole
#let _atoms(c) = {
  if type(c) == str { return c.split(" ").map(w => [#w]).intersperse([ ]) }
  if c.has("children") { return c.children.map(_atoms).flatten() }
  if c.has("text") and type(c.text) == str and c.func() == text {
    return c.text.split(" ").map(w => [#w]).intersperse([ ])
  }
  (c,)
}

#let initial(letter, lines: 4, font: read("fonts/lmroman10-bold.otf", encoding: none), gap: 0.6em, body) = layout(size => {
  let cfg = _config.get()
  // one line pitch and the cap height, measured in the current font
  let one = measure(block(width: 10em)[H]).height
  let two = measure(block(width: 10em)[H \ H]).height
  let pitch = two - one
  let side = pitch * (lines - 1) + one
  let beside = size.width - side - gap
  let seed = if cfg.seed == none { here().page() * 97 + 1030 } else { cfg.seed * 7919 + here().page() * 131 }
  let tile = _img(_plugin.initial(bytes(json.encode((
    letter: letter, side: _cm(side), seed: seed,
  ) + _colors(cfg), pretty: false)), font), width: side + 1.2pt, height: side + 1.2pt)   // 0.6 bp of margin all round
  // the most words that still fit in `lines` lines beside the tile
  let atoms = _atoms(body)
  let fits(k) = measure(block(width: beside, par(atoms.slice(0, k).join()))).height <= side + pitch * 0.25
  let (lo, hi) = (0, atoms.len())
  while lo < hi {
    let mid = calc.quo(lo + hi + 1, 2)
    if fits(mid) { lo = mid } else { hi = mid - 1 }
  }
  // break at a space, and end the block with a justified line
  let k = lo
  while k > 0 and k < atoms.len() and atoms.at(k) != [ ] { k -= 1 }
  if k == 0 { k = lo }
  // ending where the word begins, as \llap set it in the LaTeX version
  let hidden = box(width: 0pt, move(dx: -measure(letter).width, text(fill: rgb(0, 0, 0, 0), letter)))
  block(spacing: par.leading, {
    place(top + left, dx: -0.6pt, dy: -0.6pt, tile)
    pad(left: side + gap, par(hidden + atoms.slice(0, k).join() + if k < atoms.len() { linebreak(justify: true) }))
  })
  if k < atoms.len() {
    set par(first-line-indent: 0pt)
    atoms.slice(k).join()
  }
})

// ---------------------------------------------------------------- one primitive
// draw("flower", size: 1.2): any primitive of flora (petal flower lily blossom
// floret bud bell leaf leaflet cyme sprig umbel seedhead tendril sparkle rosette
// hedera butterfly bee snail bird) or the sprays blossoms, bells, budding, on
// its own and cropped to what it draws; opts as in flora.rs, e.g.
// draw("leaf", size: 2, opts: (depth: 0.6)); ghost: 0.62 draws it pale;
// angle turns it. With origin: true it takes no space and its origin (its base
// or center, as in flora.rs) is where it stands, as \florilegiumdraw in a TikZ scope.
#let draw(name, size: 1, opts: (:), seed: 1, ghost: none, angle: 0deg, origin: false) = context {
  let cfg = _config.get()
  let args = bytes(json.encode((
    name: name, size: size, opts: opts, seed: seed, ghost: ghost, angle: angle / 1deg,
  ) + _colors(cfg), pretty: false))
  let (x0, y0, x1, y1) = json(_plugin.extent(args))
  let img = _img(_plugin.draw(args), width: (x1 - x0) * 1pt, height: (y1 - y0) * 1pt)
  if origin { box(width: 0pt, height: 0pt, place(dx: x0 * 1pt, dy: -y1 * 1pt, img)) } else { img }
}

// ---------------------------------------------------------------- a headpiece
// headpiece(width, height): a plum spray in the manner of the sumi-e style, as
// an image of that size, for the empty space above a chapter's title. A limb
// comes in from the left edge (flip: true, the right) and reaches across, so
// the image is best placed at the edge of the paper. Its colors are the
// sumi-e style's unless ink, accent, blossom are given; it needs no
// florilegium.with(..).
#let headpiece(width, height, seed: 1, age: 0.5, flip: false, ink: auto, accent: auto, blossom: auto) = {
  let pick(v, k) = if v == auto { _styles.at("sumi-e").at(k) } else { v }
  let cfg = (ink: pick(ink, "ink"), accent: pick(accent, "accent"), blossom: pick(blossom, "blossom"))
  context {
    let (w, h) = (width.to-absolute(), height.to-absolute())
    let args = (w: _cm(w), h: _cm(h), seed: seed, age: age, flip: flip) + _colors(cfg)
    _img(_plugin.spray(bytes(json.encode(args, pretty: false))), width: w, height: h)
  }
}
