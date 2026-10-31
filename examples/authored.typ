// authored.typ -- the authored composition:
//   typst compile --root .. authored.typ
// The margin is drawn by the plugin (authored.rs); the body text describes how.
#import "common.typ": *
#show: florilegium.with(composition: "authored", header: running-head[3#h(1em)Procedural ornament])
#show: demo

#include "text-methods.typ"
#include "text-sharing.typ"
