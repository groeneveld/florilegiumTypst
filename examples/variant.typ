// variant.typ -- one page, one seed; used by readme-tools/sheets.py:
//   typst compile --root .. --input seed=7 [--input ink=5B2A1F] [--input age=0.8]
//     [--input style=sumi-e] [--input blossom=E8A0B4] variant.typ
#import "common.typ": *
#let seed = int(sys.inputs.at("seed", default: "7"))
#let style = sys.inputs.at("style", default: "delft")
#let hex(k) = if k in sys.inputs { rgb("#" + sys.inputs.at(k)) } else { auto }
#show: florilegium.with(seed: seed, style: style, ink: hex("ink"), blossom: hex("blossom"),
  footer: none, header: running-head[3#h(1em)Procedural ornament])
#show: demo
#if "age" in sys.inputs { age(float(sys.inputs.age)) }
#counter(heading).update((3, 1))
== One seed, one garden

#if style == "sumi-e" [
This page was painted from seed #raw(str(seed)). Nothing in the margin was placed by hand:
two or three limbs come in from the edge of the paper and zigzag through the margin,
turning back where they would leave it and stopping where they meet another; shoots fork
from their joints, and blossoms, half-open flowers and buds sit along the wood, crowded on
the young twigs and sparse on the old. The same seed always gives the same branch; a
different seed gives a different one. Because the text on this page ends early, a limb
may also reach into the space below these lines.
] else [
This page was grown from seed #raw(str(seed)). Nothing in the margin was placed by hand:
three stems rise from a root in the corner and sway up the column, leaves hang at their
nodes, and a greedy loop then finds the largest remaining hole, chooses a motif that fits
it and grows a branch to it from the nearest stem, until only small gaps are left. The same
seed always gives the same garden; a different seed gives a different one. Because the text
on this page ends early, the garden also grows into the space below these lines.
]
#textend()
