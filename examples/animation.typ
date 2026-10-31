// animation.typ -- the growth of one garden, one page per frame; used by
// readme-tools/animate.py:
//   typst compile --root .. --input seed=7 --input frames=260 [--input from=1 --input to=260]
//     [--input explain=1] animation.typ 'frames-{0p}.png'
#import "common.typ": *
#let input(k, d) = sys.inputs.at(k, default: d)
#let seed = int(input("seed", "7"))
#let frames = int(input("frames", "260"))
#let (first, last) = (int(input("from", "1")), int(input("to", str(frames))))
#let explain = input("explain", "") != ""
#show: florilegium.with(seed: seed, footer: none, header: running-head[3#h(1em)Procedural ornament])
#show: demo

#for k in range(first, last + 1) {
  // every frame is the same page: same number, same text, same seed. The
  // frames flow from page to page (a column break, not a page break), so they
  // are laid out one after another and share the one recording of the garden;
  // the page counter goes back to 0 at the end of a frame and steps to 1 with
  // the next page
  if k > first {
    counter(page).update(0)
    colbreak()
  }
  counter(heading).update((3, 2))
  frame(k, frames, explain: explain)
  [
    == Watch it grow

    This garden grows from seed #raw(str(seed)). Three stems rise from a root in the corner,
    leaves open at their nodes as the tips pass, and each stem ends in a flower. Then a
    greedy loop takes over: it finds the largest circle of free space within reach of a
    stem#if explain [ (dashed)], chooses a motif that fits it, and grows a branch to it from
    the nearest stem point. Big holes come first, so flowers and lilies land early; the
    last small gaps get leaflets, buds and florets. The final frame is exactly the page
    that this seed always gives.
    #textend()
  ]
}
