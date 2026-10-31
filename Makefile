TYPST   ?= typst
CARGO   ?= cargo
PYTHON  ?= python3
SEEDS   ?= 1 2 3 4 5 6
DOCS     = authored grown showcase specimen sumi-e
DEPS     = florilegium.typ florilegium.wasm examples/common.typ $(wildcard examples/text-*.typ)
WASM     = plugin/target/wasm32-unknown-unknown/release/florilegium.wasm

# the example documents, about a second a page; one compilation each
all: $(DOCS:%=examples/%.pdf)

examples/%.pdf: examples/%.typ $(DEPS)
	$(TYPST) compile --root . $< $@

# one page for one seed:  make seed-7  ->  build/seed-7.pdf
seed-%: examples/variant.typ $(DEPS)
	mkdir -p build
	$(TYPST) compile --root . --input seed=$* $< build/seed-$*.pdf

# the README's images, as WebP: seeds (animated), one seed in six inks, one seed through
# the seasons; the same seeds in the sumi-e style (python3 + Pillow; Typst writes the PNGs).
# One plum in three blossom inks is not in the README.
sheets: examples/variant.typ $(DEPS) readme-tools/sheets.py
	$(PYTHON) readme-tools/sheets.py --seeds $(SEEDS)
	$(PYTHON) readme-tools/sheets.py --seed 6 --inks 2456C4 5B2A1F 2F6B4F 8E1B3A 4B3B8F 1D6E7A
	$(PYTHON) readme-tools/sheets.py --seed 6 --ages 0 0.33 0.67 1
	$(PYTHON) readme-tools/sheets.py --style sumi-e --seeds $(SEEDS)
#	$(PYTHON) readme-tools/sheets.py --style sumi-e --seed 6 --blossoms C8302C E8A0B4 7A3B69

# the growth of one seed:  make grow-7  ->  build/grow-7.gif and .mp4 (MP4 needs ffmpeg);
# EXPLAIN=1 also marks the free disc each branch grows to  ->  build/grow-7-explain.*
grow-%: examples/animation.typ $(DEPS) readme-tools/animate.py
	$(PYTHON) readme-tools/animate.py --seed $* $(if $(EXPLAIN),--explain)

# the growth animations of seed 6 (all four visitors); not in the README
# animation:
# 	$(MAKE) grow-6
# 	$(MAKE) grow-6 EXPLAIN=1
# 	cp build/grow-6.gif images/growth.gif
# 	cp build/grow-6-explain.gif images/growth-explain.gif

# the README's page preview, from the example PDFs (Pillow); pages(doc) puts all
# pages of a document side by side in images/preview-<doc>.webp
pages = $(TYPST) compile --root . --ppi 45 examples/$(1).typ 'build/$(1)-{p}.png' && \
	$(PYTHON) -c "from PIL import Image; import glob; \
	  ims = [Image.open(f).convert('RGB') for f in sorted(glob.glob('build/$(1)-[0-9].png'))]; w, h = ims[0].size; \
	  s = Image.new('RGB', (len(ims) * (w + 8) + 8, h + 16), (235, 235, 240)); \
	  [s.paste(im, (8 + i * (w + 8), 8)) for i, im in enumerate(ims)]; s.save('images/preview-$(1).webp', quality=85, method=6)"
preview: examples/sumi-e.pdf
	mkdir -p build images
#	$(TYPST) compile --root . --pages 1 --ppi 60 examples/authored.typ images/preview-authored.png
#	$(TYPST) compile --root . --pages 1 --ppi 60 examples/grown.typ images/preview-grown.png
#	$(call pages,showcase)
	$(call pages,sumi-e)

# the plugin: Rust -> WASM (needs cargo and `rustup target add wasm32-unknown-unknown`).
# The documents only need florilegium.wasm, which is kept in the repository, so
# they compile without Rust; after a change to plugin/src, this rebuilds it.
plugin:
	cd plugin && $(CARGO) build --release --target wasm32-unknown-unknown
	cp $(WASM) florilegium.wasm

# a regression check for changes to the plugin: the operators of a few pages
# (seeds, spill, seasons, marks, an animation frame, the authored page), saved by
# `make baseline` before a change and compared byte for byte by `make check` after
CASES = "grown 1" "grown 6" "grown 7 14.5" "grown 9 18 1" "grown 5 0 0.2" "authored 7"
DUMP  = plugin/target/release/examples/dump
dump:
	cd plugin && $(CARGO) build --release --example dump
baseline: dump
	rm -rf build/baseline && mkdir -p build/baseline
	@for a in $(CASES); do $(DUMP) $$a > "build/baseline/$$a.txt"; done
	@MARKS="h1@20.5,h2@14,b@9,blily@6" FRAME=30/60/explain $(DUMP) grown 6 > "build/baseline/marks-frame.txt"
	@echo "saved $$(ls build/baseline | wc -l | tr -d ' ') pages in build/baseline"
check: dump
	@test -d build/baseline || { echo "no baseline: run make baseline before the change"; exit 1; }
	@for a in $(CASES); do $(DUMP) $$a | cmp -s - "build/baseline/$$a.txt" && echo "$$a: identical" || { echo "$$a: DIFFERENT"; exit 1; }; done
	@MARKS="h1@20.5,h2@14,b@9,blily@6" FRAME=30/60/explain $(DUMP) grown 6 | cmp -s - "build/baseline/marks-frame.txt" \
	  && echo "marks, frame 30/60: identical" || { echo "marks, frame: DIFFERENT"; exit 1; }

clean:
	rm -rf build

.PHONY: all sheets preview plugin dump baseline check clean
