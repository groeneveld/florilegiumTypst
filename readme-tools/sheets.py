#!/usr/bin/env python3
"""Render a contact sheet: compile examples/variant.typ once per variant with Typst, which
writes each page as a PNG, and write a contact sheet as WebP (for seeds, an animated WebP
with one frame per seed instead).

    python3 readme-tools/sheets.py --seeds 1 2 3 4 5 6     -> images/seeds.webp (animated)
    python3 readme-tools/sheets.py --seed 6 --inks 2456C4 5B2A1F   -> images/inks.webp
    python3 readme-tools/sheets.py --seed 6 --ages 0 0.5 1         -> images/seasons.webp
    python3 readme-tools/sheets.py --style sumi-e --seeds 1 2 3    -> images/sumi-e-seeds.webp (animated)
    python3 readme-tools/sheets.py --style sumi-e --seed 4 --blossoms C8302C E8A0B4
                                                                   -> images/sumi-e-blossoms.webp
Options: [--dpi 50] [--jobs 4]
"""
import argparse, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BLUE = (36, 86, 196)
VERMILION = (200, 48, 44)

def build(v, dpi, style):
    """v = (name, label, seed, ink, age, blossom): one page -> build/<name>.png"""
    name, _, seed, ink, age, blossom = v
    png = os.path.join(ROOT, "build", f"{name}.png")
    cmd = ["typst", "compile", "--root", ".", "--ppi", str(dpi), "--input", f"seed={seed}",
           "--input", f"style={style}"]
    if ink: cmd += ["--input", f"ink={ink}"]
    if blossom: cmd += ["--input", f"blossom={blossom}"]
    if age is not None: cmd += ["--input", f"age={age}"]
    r = subprocess.run(cmd + ["examples/variant.typ", png], cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr[-3000:])
        raise SystemExit(f"{name}: typst failed")
    return png

def season(a):
    a = float(a)
    return "spring" if a < 0.17 else "early summer" if a < 0.5 else "late summer" if a < 0.83 else "autumn"

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds", nargs="+", type=int, default=[1, 2, 3, 4, 5, 6])
    ap.add_argument("--seed", type=int, default=6, help="the seed of the ink and season sheets")
    ap.add_argument("--inks", nargs="+", help="HTML colors: one page per ink -> images/inks.webp")
    ap.add_argument("--ages", nargs="+", help="ages 0..1: one page per age -> images/seasons.webp")
    ap.add_argument("--blossoms", nargs="+", help="HTML colors: one page per blossom ink (sumi-e) -> images/sumi-e-blossoms.webp")
    ap.add_argument("--style", default="delft", help="delft or sumi-e (the sheet's name gets the prefix sumi-e-)")
    ap.add_argument("--dpi", type=int, default=50)
    ap.add_argument("--jobs", type=int, default=4)
    a = ap.parse_args()
    if a.inks:
        kind, cols = "inks", 3
        variants = [(f"ink-{c}", "#" + c.upper(), a.seed, c, None, None) for c in a.inks]
    elif a.blossoms:
        kind, cols = "blossoms", len(a.blossoms)
        variants = [(f"blossom-{c}", "#" + c.upper(), a.seed, None, None, c) for c in a.blossoms]
    elif a.ages:
        kind, cols = "seasons", len(a.ages)
        variants = [(f"age-{x}", season(x), a.seed, None, x, None) for x in a.ages]
    else:
        kind, cols = "seeds", 3
        variants = [(f"seed-{s}", f"seed {s}", s, None, None, None) for s in a.seeds]
    if a.style != "delft":
        variants = [(f"{a.style}-{v[0]}",) + v[1:] for v in variants]
    os.makedirs(os.path.join(ROOT, "build"), exist_ok=True)
    os.makedirs(os.path.join(ROOT, "images"), exist_ok=True)
    with ThreadPoolExecutor(max_workers=a.jobs) as ex:
        paths = list(ex.map(lambda v: build(v, a.dpi, a.style), variants))
    frames = []
    for v, path in zip(variants, paths):
        im = Image.open(path).convert("RGB")
        d = ImageDraw.Draw(im)
        try:
            font = ImageFont.truetype("DejaVuSans.ttf", max(12, im.width // 30))
        except OSError:
            font = ImageFont.load_default()
        c = v[3] or v[5]
        color = tuple(int(c[i:i + 2], 16) for i in (0, 2, 4)) if c else BLUE if a.style == "delft" else VERMILION
        d.text((im.width - 10, im.height - 10), v[1], fill=color, font=font, anchor="rd")
        frames.append(im)
    cols = min(cols, len(frames)); rows = (len(frames) + cols - 1) // cols
    w, h = frames[0].size
    sheet = Image.new("RGB", (cols * w + (cols + 1) * 8, rows * h + (rows + 1) * 8), (235, 235, 240))
    for i, im in enumerate(frames):
        sheet.paste(im, (8 + (i % cols) * (w + 8), 8 + (i // cols) * (h + 8)))
    out = kind if a.style == "delft" else f"{a.style}-{kind}"
    if kind == "seeds":
        # the README shows the seeds animated, not as a sheet
        # sheet.save(os.path.join(ROOT, "images", f"{out}.webp"), quality=85, method=6)
        frames[0].save(os.path.join(ROOT, "images", f"{out}.webp"), save_all=True, append_images=frames[1:],
                       duration=900, loop=0, quality=80, method=6)
    else:
        sheet.save(os.path.join(ROOT, "images", f"{out}.webp"), quality=85, method=6)
    print(f"wrote images/{out}.webp:", ", ".join(v[1] for v in variants))

if __name__ == "__main__":
    main()
