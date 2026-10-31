// showcase.typ -- what the grown margin does besides growing: it reads the
// text. One compilation:
//   typst compile --root .. showcase.typ
#import "common.typ": *
#show: florilegium.with(sections: true, aging: true, header: running-head[A year in the margin])
#show: demo
#set heading(numbering: "1.1")
#show heading.where(level: 1): set text(size: 14pt)

= Spring

#initial("E", lines: 5)[very page of this document grows its own garden, but the gardens here also read the text they surround. This first page is spring: the plant is young, its leaves are small and pale, and it is mostly in bud; the last page is autumn. The document turns the garden on with one show rule and three options, and the sections that follow show what each of them does, along with a few things that need no option at all:]

#align(center, ```typ
#show: florilegium.with(sections: true, aging: true)
```)

The letter that opens this paragraph is one of them. It was set with
`#initial("E", lines: 5)[...]`, which reads the outline of the letter from an OpenType font (Latin
Modern Bold, the OpenType twin of Computer Modern, unless another is given), paints it like
a petal, and sets it in a tile five lines tall. A vine crosses the tile and passes over and
under the strokes of the letter in turn; where it passes over, a thin halo of paper parts
it from the letter. Leaves go where they fit and small flowers into the largest gaps, which
for a B or an O are its counters. The letter is also set in the text, invisibly, so the
paragraph still copies and searches as _Every_.

== Flowers at the headings

With the option `sections`, every heading of the first two levels gets a flower in the
margin, beside it: a large one for a section, a spray of small ones for a subsection,
painted in the copper of the accents, the way headings were once picked out in red. The
background of every page asks the document where its headings are, and since Typst knows
the position of everything it has laid out, the garden knows the height of each heading
on the same page in the same compilation. Where the margin is already taken, by the large
flower at the top of the page for example, the flower is made smaller or left out.

== A flower where the text asks for one

The same mark can be placed by hand. `#bloom()` puts a flower in the margin beside the
line in which it stands, like this one#bloom(), and `#bloom("lily")` asks for any of the
motifs the garden is made of: `lily`, `blossoms`, `cyme`, `umbel`, `bud`, or a
`butterfly`. It costs nothing on pages that have no marks and does not move the text.

= Summer

By the middle of the document the garden is in full bloom: open flowers, star lilies,
forget-me-nots curling at their tips, umbels. The option `aging` needs the page count to
know where the middle is; LaTeX reads it from the run before and asks for a second one,
while Typst knows it in the same compilation, from `counter(page).final()`.

The seasons change what grows, not where. In spring sprays of closed buds take the place of
most open flowers, the two shorter stems end in buds instead of a lily and a flower, the
leaves are smaller and paler, and there are more tendrils. Summer is the garden as it
always was, which is also what a document without the option gets.

== Visitors

Most pages have visitors: one or two butterflies and bees in the open spaces, a snail
crawling up one of the main stems, and on about one page in five a bird perched on a twig
of its own. They are drawn from their own random stream, so a page without them is exactly
the page it would have been, and the option `visitors: false` leaves them all out. In the
growth animation the butterflies and bees fly in, the bird glides onto its twig and the
snail crawls up its stem until the last frame.

== Watching it grow

Every garden can be watched as it grows. `make grow-7` records the growth of seed 7
while the page is laid out: every stem is also drawn as shorter prefixes of itself, and
every motif at a few smaller sizes with the same random draws. Each frame is then a page
that writes the pieces whose time has come, in their original painting order, so the last
frame is exactly the finished page. Typst writes the frames as images itself, and a short
script turns them into a GIF and an MP4; `EXPLAIN=1` adds a dashed circle for each free
space the loop picks, just before a branch grows to it. The stems rise first and the leaves
open as their tips pass; then the large holes fill, slowly, and the small ones last,
quickly.

== One seed, one garden

Nothing changes from one run to the next. A page's seed comes from its page number, or
from the option `seed: 7` for all of them, and every decision is drawn from random streams
derived from it: the layout from one, each primitive from a fork of another, and the
visitors and the season each from their own. A page without visitors, or without an age,
is therefore exactly the page it would have been without those features. The same source
always gives the same PDF.

== Colors

The whole palette comes from one color, called `ink`. The option `ink: rgb("#5B2A1F")`
paints the garden in sepia, `"#2F6B4F"` in green, `"#8E1B3A"` in crimson: the ramp from the
darkest stroke to the palest wash is computed from that one hue, so the light end of it
never turns gray. A second color, `accent`, is the copper of the hairlines, the sparkles
and the flowers' eyes, and in autumn of some of the leaves.

= Autumn

On the last page the garden goes to seed. Fewer flowers open; the shorter stems end in seed
heads, dandelion clocks whose seeds drift away from them; loose petals have fallen into the
space below the flowers; and some of the leaves have turned copper. Without the option
`aging`, `#age(0.8)` sets the age by hand instead, from 0, spring, through 0.5, summer, to
1, autumn, for all the pages that follow.

The text ends here, early on its page, and `#textend()` tells the garden so: it grows into
the space below these lines as well.
#textend()
