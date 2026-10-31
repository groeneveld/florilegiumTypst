# florilegium — handoff notes

Procedurally generated Delft-blue floral page margins for Typst. All geometry is computed
in a Rust plugin (WASM) and written as raw PDF path operators into a one-page PDF that
Typst embeds behind the page; Typst does no arithmetic. Two compositions share one
package: an **authored** one (hand-placed stems, leaves and branch targets, parameters
sampled) and a **grown** one (layout computed by a greedy space-filling loop). Same
primitives, same motif table. No images, no assets besides the font for initials. The
grown margin also reads the text (flowers at headings and marked lines, seasons over the
document), pages get visitors, and paragraphs can open with a decorated initial;
`examples/showcase.typ` demonstrates all of it.

The project began as a LuaLaTeX package (Lua drawing code, plus a PostScript port); the
plugin is a line-for-line port of that Lua, byte-identical when it replaced it. Both are in
the git history only.

## Files

| file | what |
|---|---|
| `florilegium.typ` | the Typst layer: `florilegium.with(..)` (sets the page and its background; `style: "delft" \| "sumi-e"` picks ink, accent and blossom colors), `bloom`, `age`, `textend`, `initial`, `frame`, `draw`, `headpiece` (a sumi-e plum spray in a band, as an image). Reads marks and the text end with `query`/`locate`, the page count with `counter(page).final()`, and calls the plugin. |
| `florilegium.wasm` | the built plugin (`make plugin`), kept in the repository so documents need no Rust. |
| `fonts/lmroman10-bold.otf` | Latin Modern Roman Bold (GUST Font License), the default font of `initial`. |
| `typst.toml` | package manifest; the package is `florilegium.typ`, `.wasm`, `fonts/`. |
| `plugin/src/ink.rs` | drawing core: mulberry32 rng (`Cell` state, so closures can draw), 1-D gradient noise/fbm, OKLCH tone ramp (plus an optional blossom ramp, used inside `in_bloom`, which flora's flower primitives wrap themselves in), affine transform stack, path building (`smooth` = Catmull-Rom as Béziers, `taper`, `ribbon`, `blade`, `blob`), fill/stroke/clip with ExtGState opacity + multiply, numbered layers, `flush()`; recording/`emit` for the animation. Global state in a thread-local, as in ink.lua. |
| `plugin/src/flora.rs` | the primitives: `petal flower lily blossom floret bud bell leaf leaflet stem cyme sprig umbel seedhead tendril sparkle rosette hedera ghost`, the visitors `butterfly bee snail bird`, `branch_path`; options in one struct `O`; `draw(name, ..)` by name. |
| `plugin/src/grow.rs` | grown composition: regions, grid, stems, marks, visitors, leaves at nodes, branch loop, falling petals, filler, footer tail; seasons; animation events and `schedule`. Exports the **motif table** (`MOTIFS`, `motif(name)`) and layer numbers used by `authored.rs`. |
| `plugin/src/authored.rs` | authored composition: the hand-placed score for A4 with a 68 mm left margin. |
| `plugin/src/sumi.rs` | the plum composition (`style: "sumi-e"`): zigzag limbs that turn back at the region's edge, children at joints, young shoots; segments bow between angular joints (a hash of the joints, no rng draw); painted as dry brush (gray washes of uneven strength, broken dark bristle strands, moss dots); five-petal blossoms, half-open flowers and buds in the blossom ink; twigs to the marks; falling petals; its own header (brush rule + red seal). Its own rng stream (`seed*7919+17`), so it never moves the grown pages. `spray`: the same tree grown into a w×h band, entering from one side (a headpiece). |
| `plugin/src/initial.rs` | decorated initials: glyph outline via ttf-parser, even-odd inside test, clearance grid, the vine over/under the letter, leaflets, flowers in the gaps, tile frame. |
| `plugin/src/lib.rs` | the plugin's functions, JSON in, a one-page PDF out: `page`, `record`/`frame` (animation, through `plugin.transition`), `initial` (+ font bytes), `draw`/`extent`, `spray`. Also the header (ivy leaf + head rule) and the page seed. |
| `plugin/examples/` | `dump.rs` (a page's operators, for `make baseline`/`check`), `bench.rs` (native ms per page), `pagepdf.rs` (the plugin's PDF for one JSON). |
| `examples/` | `authored`, `grown`, `showcase`, `specimen`, `sumi-e` (the plum style, seed 4 by default: its last page reaches under the text; a headpiece), `variant` (one page for a given seed, ink, age, style and blossom; `make seed-7`), `animation` (one page per frame); `common.typ` (look, theorem blocks); body text in `text-*.typ`: it describes the code, so **update it when the method changes**. |
| `readme-tools/sheets.py`, `readme-tools/animate.py` | the README's images as WebP (`images/seeds.webp` and, with `--style sumi-e`, `sumi-e-seeds.webp`, both animated; `inks.webp`, `seasons.webp`) and the growth GIF/MP4 in `build/`, from Typst-written PNGs. `images/` holds only what the README shows; makers of other images are commented out. |

## Build

```sh
make                        # examples/{authored,grown,showcase,specimen,sumi-e}.pdf (~1 s a page)
make seed-7                 # one page for seed 7 -> build/seed-7.pdf
make sheets                 # seeds, inks, seasons, sumi-e seeds (python3 + Pillow, ~20 s)
make grow-6 [EXPLAIN=1]     # ~1 min: build/grow-6.gif (+ .mp4 with ffmpeg)
make preview                # images/preview-sumi-e.webp
make plugin                 # rebuild florilegium.wasm (cargo + rustup target wasm32-unknown-unknown)
make baseline; make check   # before/after a plugin change: same operators, byte for byte (cargo)
```

Fixed seed in a document: `florilegium.with(seed: 7)`; the examples take
`--input seed=7`. Any positive integer works.

Look at output with `pdftoppm -png -r 100 file.pdf page` (or `typst compile … out.png
--ppi 100`) and read the PNG; for fine detail use `-r 300` with `-x -y -W -H` crops. Always
look before declaring a change good. `scratch/` (git-ignored) is the place for test documents.

Speed: natively ~25 ms a page (`cargo run --release --example bench`); under Typst's WASM
interpreter ~1 s. A page is ~3 MB of operators, ~0.6 MB in the PDF.

## Conventions and traps

- **US spelling everywhere** (color, center, gray, meter) — code, comments, docs.
- **Regression check.** Before touching a primitive, `grow.rs` or `authored.rs`, run
  `make baseline`; after, `make check`. A refactor must leave every page byte-identical; a
  change meant to alter the drawing shows up as DIFFERENT, so look at the pages and then
  take a new baseline. Then `make plugin`, since documents use the committed `.wasm`.
- **Keep the rng draw order.** Every `rng`/`layout` draw shifts everything after it, so
  reordering argument evaluation, making a lazy default (`unwrap_or_else`) eager, or adding
  a draw changes all later gardens. The code keeps Lua's habits: floored `%` (`ink::fmod`,
  `rem_euclid`) and some 1-based index arithmetic (`floor(#pts*f)` then `pts[i-1]`).
- **Output is a PDF page, not SVG.** The plugin writes uncompressed (Typst compresses).
  SVG cost 6× the size: Typst re-encodes coordinates as f32 with 8–9 digits, and
  `opacity`/`mix-blend-mode` become a transparency group per element.
- **Line widths are absolute bp** (points are transformed in Rust, not with `cm`). Anything
  that should scale with a motif is a filled shape (`taper`, `ribbon`); `flora::stem`
  switches to a stroke below ~0.7 bp.
- **Size budget.** Output, not time, is the constraint. Coordinates are printed `%.1f` bp
  (`ink::tenth`, exact, with a fast path); a path used for fill and clip is emitted once
  (`clip()` then `flood()`); streaks are `taper`; florets below 0.7 mm are a 10-point polygon.
- **Level of detail**: primitives check `ink::unit()` (cm per local unit) and
  `ink::get_lift()` (ghost) and skip specks/veins when small or pale.
- **Determinism**: every primitive takes an rng; layout code draws from `layout` and hands
  primitives `rng.fork()`, so a change inside a primitive does not move the layout. Don't
  iterate a `HashMap` where the order feeds an rng.
- **Animation replays draws.** With `anim`, `Garden::place` calls each closure several times
  (smaller scales first) and restores `rng` and `layout` with `save()`/`load()` in between.
  A closure may only draw from those two (fork inside the closure, not before it), or the
  last frame stops matching the page. The last frame uses t = ∞, and `Tag::visible` treats
  `to = ∞` as open (∞ < ∞ is false).
- **Separate streams keep old pages stable.** Visitors draw from `Rng::new(seed*7919+101)`,
  the season's additions from `+211`, the authored visitors from `+103`.
- **Typst layout passes**: the background runs in every pass; in the first nothing has a
  position, so `_background` returns nothing until the sentinel `<florilegium-done>` (end of
  the document) can be found, or every garden is grown twice.
- **Parallel pages**: pages split by `pagebreak()` are laid out in parallel and don't share
  memoized plugin calls; `animation.typ` uses `colbreak()` so one
  `plugin.transition(record)` serves all its frames. `counter(page)` in the background is
  read at the page's start, the footer's at its end (hence `update(0)` before the break).
- **Positions**: inline `metadata` reports its baseline; a heading's position is the top of
  its first line, so heading marks add 0.25 cm.
- **Initials**: the tile's PDF has 0.6 bp of margin all round (the frame stroke sits on the
  edge); `initial` places it at −0.6 pt. The invisible letter ends where the word begins
  (as `\llap` did), or text extraction reads "Eery".
- **Page geometry** comes from `florilegium.with(page-margin:, headsep:, footskip:)`; a
  document's own `set page` would move the text without telling the garden.

## How the growth works (grow.rs)

1. Regions: margin column, a footer band along the bottom (to ~45% of the text width),
   and on the last page the rectangle under the text.
2. Stems: three from a root near the bottom-left corner; x(y) = c + A·fbm(y/λ + φ), blended
   into the root over 4 cm. Tops at the page top (hero flower), 60–72% (lily), 36–48%
   (flower). A spiral scroll at the root; a footer stem to the right. On the last page,
   Bézier arcs from a main stem into the spill, each ending in a flower.
3. Ghosts (1–2 pale flowers, layer 1) get a weak proxy so the growth leaves them some room.
4. Leaves at nodes of every "leafy" stem (alternating, 35–60°, smaller toward the top; the
   footer gets leaflets that thin out), only where `free` allows.
5. Grid (2 mm), `free` = largest disc that fits (proxies count at κ=0.85 of their radius;
   stems are chains of small discs) and `sdist` = distance to the nearest stem point.
   Updates only visit lattice cells near the new disc (a dense lattice index).
6. Branch loop: cell with the largest `free·U(0.85,1.15)` within REACH (3 cm) of a stem;
   pick a motif by weight `w/(1+count/cap)` (hard caps only for flower, lily, umbel,
   bells); scale so r(s) ≈ 0.95·free; axis = away from the nearest stem point + `upright`
   lean; `flora::branch_path` from that stem point; the branch's points become stem points.
   Stop below SMALLEST (3.6 mm).
7. Filler: holes ≥ 1.7 mm within 7.5 mm of a stem get a leaflet pair, a bud or a floret
   cluster (capped); far holes get at most 6 copper accents; no dust.
8. Footer tail: copper hairline + sparkle. Layers: 1 ghost, 2 stems, 3 leaves, 4 airy,
   5 blooms, 6 accents (7 overlay, animation only; 100 the header); a safety clip a little
   outside the regions.
9. Marks (right after the ghosts): a flower beside each heading (copper, via
   `ink::in_accent`) or `bloom(motif)`, at the column's edge, shrunk or left out where
   there's no room. Then visitors on the stems: a snail on a main stem (40% of pages), a
   bird on its own twig (22%). Butterflies and bees are motif rows with `fly` (no branch,
   fixed caps of 2), allowed per page by the visitor stream.
10. Seasons (`age`; None = summer): `season(spring, summer, autumn)` blends a value; motif
   rows carry season weight multipliers (`budding` only in spring, `seedhead` only in
   autumn); the shorter stems end in buds or seed heads; young leaves are smaller and paler
   (ghost lift); autumn turns some leaves copper and drops petals below the flowers.
11. Animation (`anim`): every step is an `Event` (a stem that grows or a motif that opens),
   in a phase (stems, ghost, marks, branch, fall, fill, accent, tail) or starting at a
   fraction `at` of its parent's growth. Output is tagged (`ink::Tag`); stems are also drawn
   as prefixes every GROWSTEP with a pointed tip, motifs at the BLOOM scales. `schedule()`
   times the events (~13 s); `lib.rs` records once (`record`, kept by `plugin.transition`)
   and `frame` emits the visible pieces in painting order. The explained version adds a
   dashed disc per branch. A `path` moves a motif instead of opening it (visitors fly in,
   petals fall, the snail crawls with `till_end`).

To add a motif: write the primitive in `flora.rs` (origin at its base or center, axis +y,
takes `&Rng`), add a row to `MOTIFS` in `grow.rs` with `r(s)` (proxy radius), `off(s)`
(center back to the origin along the axis), `upright`, `layer`, `w`, `max`, optionally
`hard`/`hang`/`fly` and season multipliers, and `draw(rng, s, p)`; add it to
`flora::draw` for `#draw`. `authored.rs` can then place it by name, and `#bloom("name")`
can ask for it. Adding a motif row changes the weights, so every grown page changes: take
a new baseline afterwards.

## Ideas not done

- Proxies of 2–3 discs for long motifs (leaves, cymes) instead of one disc.
- Read free space on every page (figures, display equations), not just the last.
- Leaf tips that curl over and show a paler underside, as in the reference image.
- Heading baselines from the heading itself instead of top + 0.25 cm.
- Initials (and `#draw`) in the plum style: a brush-stroke letter with a blossom twig.
- Initials in the authored composition's style for the first page, and a bigger, framed
  title-page tile.
- A mirrored right-margin variant for two-sided documents.
- Form XObjects for repeated sub-shapes (florets) to cut the PDF size further.
- A density/airiness option (SMALLEST, FILLMIN, caps) exposed in `florilegium.with`.
- Publish to Typst Universe (the manifest is there; Universe wants the package files alone).
