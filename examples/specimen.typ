// specimen.typ -- one of every primitive, drawn with draw():
//   typst compile --root .. specimen.typ
#import "../florilegium.typ": *
#show: florilegium.with(margin: false, page-margin: (left: 1cm, right: 1cm, top: 1cm, bottom: 1cm), footer: none)
#set text(font: "New Computer Modern", size: 8pt, fill: luma(130))

// TikZ's frame: centimeters from the lower-left corner of a 19 x 27.5 cm box
#let H = 27.5
#let at(x, y, name, angle: 0deg, ..args) = place(top + left, dx: x * 1cm, dy: (H - y) * 1cm,
  draw(name, origin: true, angle: angle, ..args))
#let lab(x, y, body) = place(top + left, dx: x * 1cm - 6cm, dy: (H - y) * 1cm - 0.5em, box(width: 12cm, align(center, body)))

#block(width: 19cm, height: H * 1cm, stroke: luma(220) + 0.4pt, {
  at(3.2, 24, "flower", size: 1.9);   lab(3.2, 21.3)[flower]
  at(9, 24, "flower", size: 1.9, ghost: 0.62);   lab(9, 21.3)[ghost]
  at(15, 23.6, "lily", size: 2.0);   lab(15, 21.3)[lily]
  at(1.5, 15.5, "leaf", angle: -15deg, size: 4, opts: (lobes: 4, depth: 0.5))
  at(5.5, 15.5, "leaf", angle: 10deg, size: 4, opts: (lobes: 3, depth: 0.8, w: 0.26))
  at(9.2, 15.5, "leaf", angle: -25deg, size: 3.6, opts: (depth: 0, w: 0.18, bend: 0.3))
  at(12, 15.5, "leaf", angle: 20deg, size: 2.2, opts: (lobes: 3, depth: 0.6))
  for k in range(5) { at(14.2 + 0.9 * k, 16, "leaflet", angle: (30 * k - 60) * 1deg, size: 1.1, seed: k + 1) }
  lab(9, 15)[leaf (lobed, deeply cut, entire), leaflet]
  for k in range(4) { at(1.2 + 1.4 * k, 13, "blossom", size: 0.55, seed: k + 1) }
  for k in range(8) { at(1 + 0.7 * k, 11.6, "floret", size: 0.16, seed: k + 1) }
  at(8, 11.5, "bud", angle: -10deg, size: 1.1)
  at(9.5, 11.5, "bud", angle: 15deg, size: 0.9, seed: 2)
  at(11.6, 13.6, "bell", size: 1.3)
  at(13.6, 13.4, "bell", size: 1.0, seed: 2)
  at(16.5, 12.4, "lily", size: 1.2, seed: 2)
  lab(9, 10.7)[blossom, floret, bud, bell, lily]
  at(1.5, 6, "cyme", size: 3, opts: (side: -1))
  at(4.5, 6, "cyme", size: 2.4, opts: (side: 1), seed: 2)
  at(7.5, 6, "sprig", size: 3.2)
  at(11.5, 6, "umbel", size: 2.2)
  at(16.8, 7.5, "tendril", size: 0.8)
  at(14.6, 7.2, "seedhead", size: 0.75, opts: (side: 1))
  lab(9, 5.3)[cyme, sprig, umbel, seed head, tendril]
  at(2, 2.5, "sparkle", size: 0.3)
  at(3.5, 2.5, "rosette", size: 0.22)
  at(6, 2.5, "hedera", size: 0.55)
  lab(4.5, 0.8)[sparkle, rosette, hedera (the header fleuron)]
  at(10.2, 2.6, "butterfly", angle: 10deg, size: 1.6)
  at(12.6, 3.2, "bee", angle: -30deg, size: 0.9)
  at(14.2, 2.2, "snail", angle: 80deg, size: 1.0)
  place(top + left, line(start: (15.4cm, (H - 1.45) * 1cm), end: (18.6cm, (H - 1.6) * 1cm), stroke: luma(160) + 0.4pt))
  at(17, 1.55, "bird", size: 2.5)
  lab(14, 0.8)[visitors: butterfly, bee, snail, bird]
})
