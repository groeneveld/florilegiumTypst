// text-growth.typ -- the growth stage (used by grown.typ only)
#import "common.typ": *

=== The growth stage

This document is the answer to the exercise. Its margin is not the authored composition of
the previous section: nothing in it was placed by hand. The plugin decides where every
stem, leaf and flower goes and how large it is, and then calls the same primitives to paint
them.

#construction[Stems][
  The root is placed near the bottom-left corner of the page. Three stems rise from it; the
  $x$ coordinate of each is a smooth function of the height,
  $ x(y) = c + A n(y\/lambda + phi), wide A = 0.2 "(column width)", thick lambda in [6, 9] "cm", $
  with $n$ a two-octave gradient noise and $phi$ a phase drawn per stem, blended into the
  root over the first four centimeters. Different phases make the stems sway apart and
  cross now and then, as real stems do. The first reaches the top of the page and ends in
  the largest flower; the second stops at $60$ to $72%$ of the height and ends in a lily;
  the third at $36$ to $48%$, in a flower. A footer stem leaves the root along the bottom of
  the page. Leaves are hung at the nodes, every $1.4$ to $2.2$ cm, alternately left and
  right, $35 degree$ to $60 degree$ off the stem, $2.4$ cm long at the bottom of the page and
  $1.3$ cm at the top, and only where there is room.
]

#definition[
  A _proxy_ for a motif is a disc of radius $r(s)$, where $s$ is the motif's scale, that
  approximates the area it occupies. A point $p$ of the free region $R$ has _free radius_
  $ f(p) = min(d(p, partial R), thin min_i (norm(p - c_i) - kappa r_i)), $
  the radius of the largest disc about $p$ that stays inside $R$ and overlaps no proxy
  $(c_i, r_i)$ by more than the fraction $1 - kappa$. With $kappa = 0.85$, a leaf may tuck
  under the edge of a flower; stems are proxies too, as chains of small discs.
]

#construction[Growth][
  The region $R$ is the margin column, a band along the bottom of the page, and on the last
  page also the rectangle between the end of the text and the footer. On a grid of
  candidates spaced $2$ mm apart, each point keeps $f(p)$ and its distance to the nearest
  stem. Then, until the largest free radius falls below $3.6$ mm:
  + take the point $p$ that maximizes $f(p)$ among those within $3$ cm of a stem, with a
    random factor in $[0.85, 1.15]$ so that ties break differently on every seed;
  + choose a motif whose smallest proxy fits, by weight; a motif that has already appeared
    $k$ times has its weight divided by $1 + k\/c$, and only flowers, lilies, umbels and
    bells have a hard cap. Set its scale so that $r(s) approx 0.95 f(p)$;
  + give it an axis pointing away from the nearest stem point, leaning upward for the kinds
    that reach for the light, and grow a branch to it from that point, as in the authored
    composition. The branch becomes a stem for later branches.
  Because the largest hole is always taken first, flowers and lilies are placed early,
  sprays and leaves fill the middle, and a last pass puts pairs of leaflets, buds and small
  clusters of florets on the stems wherever a hole of more than $1.7$ mm is left next to
  one; a hole far from any stem may get one of six copper accents, and otherwise stays
  empty. That ordering is the whole reason the result looks _composed_ rather than
  scattered.
]

#remark[Adapting to the text][
  The free region is computed from the page layout, so changing the margins moves the
  garden. The end of the text is found where it happens: `#textend()` leaves an invisible
  mark at the end of the text, and the background of each page asks the document where
  that mark was placed. Typst answers such a question by laying the document out again, so
  on the last page the garden spills into the space below the words in a single
  compilation: branches arch out of the main stems into that space and each ends in a
  flower.
]

#remark[Visitors, marks and seasons][
  Butterflies and bees take holes like any motif but need no branch; a snail is put on a
  main stem and, on about one page in five, a bird on a twig of its own. Their decisions
  come from a random stream of their own, so a page without them is the page it would have
  been. A mark in the text, left by `#bloom()` or by a heading, is found by the same kind of
  question from the page's background and gets its flower before any leaf is hung. An age
  between $0$ and $1$ reweights the motifs from buds in spring to seed heads in autumn.
]

#remark[Cost][
  The layout is cheap: the grid has a few thousand points, a proxy only updates the points
  near it, and the whole loop takes a few hundredths of a second natively. Under Typst's
  WebAssembly interpreter a page takes about a second, and this document of five pages
  compiles in about five seconds.
]
