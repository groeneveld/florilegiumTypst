// text-sharing.typ -- shared closing section
#import "common.typ": *

=== Building and sharing

The package is `florilegium.typ` with a WebAssembly plugin beside it, built from a Rust
crate of six modules: `ink.rs` (geometry, color and PDF output), `flora.rs` (the
primitives), `authored.rs` and `grow.rs` (the two compositions), `initial.rs` (the
decorated initials) and `lib.rs` (the plugin's functions). It began as a port of the Lua
code of an earlier LaTeX version, line for line and draw for draw: for the same seed and
the same page, the two wrote the same content stream, byte for byte. It needs Typst 0.14 or later, which embeds PDF
pages, and one compilation, at about a second per page. To share it, the three routes that
work are, in increasing order of effort: upload the package's files to a project on
typst.app and share the link; put it in a Git repository with a continuous integration job
that compiles the PDFs on every push; or publish it as a package, so that any document can
import the garden and choose its own seed, margin and hue.

#remark[
  Two mistakes in the port cost time and are worth recording. An interval with no end is
  not the same as one that ends at infinity: the last frame of the growth animation is
  drawn at $t = infinity$, where every event's progress is infinite too, and
  $infinity < infinity$ is false, so the last frame came out empty until the test asked
  whether the interval had an end instead of comparing with it. And Typst lays a document
  out more than once when it asks where things are, and in the first pass nothing has a
  position yet: a background that calls the plugin there grows every garden twice, once
  for a page without marks and again for the real one. A sentinel at the end of the
  document, which only the second pass can find, halves the time.
]
