// grown.typ -- the grown composition:
//   typst compile --root .. grown.typ
// The layout is computed by the plugin's grow.rs and drawn by flora.rs and
// ink.rs; the body text describes all three. A fixed seed for every page:
//   typst compile --root .. --input seed=7 grown.typ
#import "common.typ": *
#let seed = if "seed" in sys.inputs { int(sys.inputs.seed) } else { none }
#show: florilegium.with(seed: seed, header: running-head[3#h(1em)Procedural ornament])
#show: demo

#include "text-methods.typ"
#include "text-growth.typ"
#include "text-sharing.typ"
#textend()
