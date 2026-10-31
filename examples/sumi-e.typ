// sumi-e.typ -- the plum style: a margin in the manner of a sumi-e painting,
// drawn by the plugin's sumi.rs. One compilation:
//   typst compile --root .. sumi-e.typ
// Every page grows from seed 4 (one whose last page reaches under the text);
// another:  --input seed=7
#import "common.typ": *
#let seed = int(sys.inputs.at("seed", default: "4"))
#show: florilegium.with(style: "sumi-e", seed: seed, sections: true, aging: true,
  header: running-head[A plum branch])
#show: demo
#set heading(numbering: "1.1")

// a headpiece above the first section: a spray entering from the right edge of the paper
#pad(right: -22mm, top: -6mm, bottom: 2mm, layout(size => headpiece(size.width, 3.2cm, seed: 3, age: 0.2, flip: true)))

= The plum branch

#initial("A", lines: 4)[s well as the Delft garden, the package paints a second kind of margin: a plum branch in the manner of a sumi-e ink painting. Black branches, gnarled and angular, enter the margin from the edge of the paper and fork into straight young shoots; small five-petal blossoms, buds and half-open flowers sit along them in vermilion, and a few petals fall. There are no leaves, and the paper between the branches is left empty, which is most of what makes the style.]

One option turns it on, and sets a warm black ink, vermilion blossoms, a vermilion accent
and a warm paper; each of them can still be given on its own:

#align(center, ```typ
#show: florilegium.with(style: "sumi-e")
```)

The style shares everything but the composition with the garden: the ink ramp, the
initials, the marks, the seasons, the way the page is read. Its composition, `"plum"`,
lives in a file of its own and draws from a random stream of its own, so adding it moved
none of the grown pages.

== How a branch grows

A branch is a zigzag polyline. Every segment turns, alternately, a little toward a goal
direction and back, the way a plum limb changes its mind at every node; where the next
joint would leave the margin, the branch turns back toward the middle of its region, and
where it runs into another branch it stops. Children leave at the joints at 25 to 60
degrees, thinner and straighter at every generation, so the old wood is gnarled and the
young shoots are whips. Between joints a segment bows a little, by an amount hashed from
the joint itself rather than drawn from the random stream, so the curve costs no draw and
moves nothing after it.

A page has two or three limbs: one from the left edge near the top, hanging down; one
lower, rising; often one from the bottom. On the last page of the text one more reaches
out under the last lines, when there is room for it.

== The dry brush

A branch is painted as a dry brush would leave it: a pale gray body of uneven strength, and
over it a few dark strands, the bristles, which break off in places and start again, so
that paper shows through the stroke. The old wood gets moss dots, small dark blots along
its edges. The blossoms are five round petals around a dark eye, in two or three washes of
the blossom ink, or a half-open cup, or a closed bud; a bud sits at most twig tips.

= The margin reads the text

The marks work as they do in the garden. With the option `sections`, every heading of the
first two levels gets a twig: from the nearest wood to the column's edge beside the
heading, zigzagging toward it, with a cluster of blossoms where it arrives, five for a
section and three for a subsection. Where there is no wood within reach, a new branch comes
in from the edge of the paper. `#bloom()` asks for the same twig beside any line, like
this one#bloom().

With the option `aging`, the seasons run from the first page to the last. For a plum tree
that means bud to blossom: early in the document more than half of the flowers are still
closed, late in it almost all are open, and more petals are falling.

== The header and the headpiece

The header is a dry-brush rule from the margin to the right edge of the text, three
strands that end unevenly, with a red seal below its right end whose strokes are cut on a
three-by-three grid. The spray at the top of the first page is `#headpiece`, the same tree
grown into a band of any width and height, entering from one side:

#align(center, ```typ
#headpiece(15cm, 3cm, seed: 3, flip: true)
```)

It needs no show rule, so it can head a chapter in any document.

= Autumn

By the last page the branch is in full flower and shedding: more petals drift below the
blossoms, and the buds are few. Because the text ends here, a last limb grows into the
space below these lines, with blossoms of its own.
#textend()
