#!/usr/bin/env python3
"""Animate the growth of one garden: compile examples/animation.typ with Typst (one page
per frame, split over parallel jobs, each of which records the garden once and writes its
frames as PNGs), and write a GIF and, if ffmpeg is installed, an MP4. The last frame is
held for a few seconds before the GIF loops.

    python3 readme-tools/animate.py --seed 7 [--explain] [--fps 20] [--seconds 13] [--jobs 8]
                             [--width 1080] [--gif-width 640] [--hold 3]

Writes build/grow-7.gif and build/grow-7.mp4 (build/grow-7-explain.* with --explain);
--out sets the path without the extension.
"""
import argparse, glob, os, shutil, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def build(job, a, frames, explain, width, framedir):
    """typst for frames a..b of `frames`: one PNG per page, renamed by frame"""
    first, last = job
    ppi = width / (595.28 / 72)                  # an A4 page `width` pixels wide
    cmd = ["typst", "compile", "--root", ".", "--ppi", f"{ppi:.3f}", "--input", f"seed={a.seed}",
           "--input", f"frames={frames}", "--input", f"from={first}", "--input", f"to={last}"]
    if explain: cmd += ["--input", "explain=1"]
    prefix = os.path.join(framedir, f"t{first:04d}")
    r = subprocess.run(cmd + ["examples/animation.typ", prefix + "-{0p}.png"], cwd=ROOT,
                       capture_output=True, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr[-3000:])
        raise SystemExit(f"frames {first}-{last}: typst failed")
    pages = sorted(glob.glob(prefix + "-*.png"))
    if len(pages) != last - first + 1:
        raise SystemExit(f"frames {first}-{last}: expected {last - first + 1} pages, got {len(pages)}")
    for k, p in enumerate(pages):
        os.replace(p, os.path.join(framedir, f"{first + k:04d}.png"))
    return last - first + 1

def gif_pillow(framedir, n, out, fps, hold, width):
    # fallback without ffmpeg: one palette from the last (fullest) frame for every frame
    ims = [Image.open(os.path.join(framedir, f"{k:04d}.png")).convert("RGB") for k in range(1, n + 1)]
    h = round(ims[0].height * width / ims[0].width)
    ims = [im.resize((width, h), Image.LANCZOS) for im in ims]
    pal = ims[-1].quantize(colors=128, dither=Image.Dither.NONE)
    q = [im.quantize(palette=pal, dither=Image.Dither.NONE) for im in ims]
    durs = [round(1000 / fps)] * (n - 1) + [round(1000 / fps + hold * 1000)]
    q[0].save(out, save_all=True, append_images=q[1:], duration=durs, loop=0)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--explain", action="store_true", help="mark the free disc each branch grows to")
    ap.add_argument("--fps", type=int, default=20)
    ap.add_argument("--seconds", type=float, default=13, help="length of the growth, before the hold")
    ap.add_argument("--hold", type=float, default=3, help="seconds on the finished garden")
    ap.add_argument("--jobs", type=int, default=min(8, os.cpu_count() or 2))
    ap.add_argument("--width", type=int, default=1080, help="frame width in pixels (MP4)")
    ap.add_argument("--gif-width", type=int, default=640)
    ap.add_argument("--out", help="output path without extension (default build/grow-SEED[-explain])")
    a = ap.parse_args()

    out = a.out or os.path.join("build", f"grow-{a.seed}{'-explain' if a.explain else ''}")
    out = os.path.join(ROOT, out)
    framedir = out + "-frames"
    shutil.rmtree(framedir, ignore_errors=True)
    os.makedirs(framedir)
    os.makedirs(os.path.join(ROOT, "build"), exist_ok=True)

    frames = max(2, round(a.fps * a.seconds))
    per = -(-frames // a.jobs)
    jobs = [(f, min(frames, f + per - 1)) for f in range(1, frames + 1, per)]
    with ThreadPoolExecutor(max_workers=len(jobs)) as ex:
        done = sum(ex.map(lambda j: build(j, a, frames, a.explain, a.width, framedir), jobs))
    print(f"rendered {done} frames of seed {a.seed} into {os.path.relpath(framedir, ROOT)}")

    pattern = os.path.join(framedir, "%04d.png")
    hold = f"tpad=stop_mode=clone:stop_duration={a.hold}"
    if shutil.which("ffmpeg"):
        ff = ["ffmpeg", "-y", "-loglevel", "error", "-framerate", str(a.fps), "-i", pattern]
        subprocess.run(ff + ["-vf", f"{hold},scale={a.gif_width}:-1:flags=lanczos,split[a][b];"
                             "[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=none:diff_mode=rectangle",
                             "-loop", "0", out + ".gif"], check=True)
        subprocess.run(ff + ["-vf", f"{hold},scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p",
                             "-c:v", "libx264", "-crf", "18", "-preset", "slow", "-movflags", "+faststart",
                             out + ".mp4"], check=True)
        print(f"wrote {os.path.relpath(out, ROOT)}.gif and .mp4")
    else:
        gif_pillow(framedir, frames, out + ".gif", a.fps, a.hold, a.gif_width)
        print(f"wrote {os.path.relpath(out, ROOT)}.gif (no ffmpeg, so no MP4)")

if __name__ == "__main__":
    main()
