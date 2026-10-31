// common.typ -- the look of the demo documents, as in the LaTeX version:
// Computer Modern at 11 pt, numbered headings, a small-caps running head, and
// amsthm-style blocks (definition, construction, remark, ...) numbered within
// the section.
#import "../florilegium.typ": *

#let demo(title: [3#h(1em)Procedural ornament], body) = {
  set text(font: "New Computer Modern", size: 11pt)
  set par(justify: true, leading: 0.55em, spacing: 0.55em, first-line-indent: 1.5em)
  set heading(numbering: "1.1.1")
  show heading: set block(above: 1.4em, below: 0.9em)
  show heading.where(level: 1): set text(size: 14pt)
  show heading.where(level: 2): set text(size: 12pt)
  show heading.where(level: 3): set text(size: 11pt)
  show raw: set text(font: "DejaVu Sans Mono", size: 0.9em)
  show raw.where(block: false): box
  set math.equation(numbering: none)
  body
}

#let running-head(title) = align(right, text(size: 9pt, smallcaps(title)))

#let _thm = counter("florilegium-demo-thm")
// Definition 3.1 (Palette). -- bold label, the note upright, the body upright or italic
#let _env(kind, italic: false) = (..args) => {
  let body = args.pos().last()
  let note = args.pos().at(0, default: none)
  if args.pos().len() < 2 { note = none }
  _thm.step()
  block(above: 1em, below: 1em, {
    strong[#kind #context [#counter(heading).get().first().#_thm.get().first()]]
    if note != none [ (#note)]
    strong[.]
    h(0.5em)
    if italic { emph(body) } else { body }
  })
}
#let definition = _env("Definition")
#let construction = _env("Construction")
#let remark = _env("Remark")
#let exercise = _env("Exercise")
#let proposition = _env("Proposition", italic: true)
#let proof(title: [Proof], body) = block(above: 1em, below: 1em, {
  emph[#title.]
  h(0.5em)
  body
  h(1fr)
  box(width: 0.6em, height: 0.6em, stroke: 0.4pt)
})
