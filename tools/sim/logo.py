#!/usr/bin/env python3
"""Sponsor logos for the reel: assets/sponsors/*.svg -> c/src/logos.h and src/logos.rs.

  python3 tools/sim/logo.py [--preview=out.png]

The LOGOS table below says how to make each logo: from a picture (SVG,
drawn with Inkscape, or PNG; ink = alpha) with how many grid rows it gets
(the grid is 16 rows high), an optional crop (fractions of the picture)
and a brightness floor; or drawn here (the ALLNET globe, whose fine wire
mesh is lost at this size, redrawn from its shape with fewer, thicker
lines).

Each logo becomes greyscale: a pixel's brightness is how much of it the
picture covers, in 2 bits (off, 1/3, 2/3, full; the LED driver applies
gamma). Lone pixels with no lit neighbour are dropped. Stored row by row,
four pixels a byte (pixel i = row * width + column, bits 2(i%4)..). In
the reel (sponsors.c / sponsors.rs) a logo that fits the grid stands in
the middle; a wider one scrolls by like text, and one taller than the
grid also pans down while it scrolls.

--preview draws the logos as the grid would show them.
"""
import math
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "assets" / "sponsors"
LEVELS = 3     # 2 bits
FLOOR = 0.15   # less coverage than this is off


def picture(file, rows, crop=None, ink="alpha", floor=FLOOR):
    img = load(ASSETS / file)
    w, h = img.size
    if crop:
        img = img.crop((round(crop[0] * w), round(crop[1] * h), round(crop[2] * w), round(crop[3] * h)))
    a = np.asarray(img).astype(float) / 255
    cover = a[..., 3]
    if ink == "white":
        cover = cover * (a[..., :3].mean(2) > 0.5)
    elif ink.startswith("#"):
        cover = outlined(a, ink)
    return quantise(cover, rows, floor)


def outlined(a, colour):
    """Letters drawn as outlines in `colour` on a solid shape (deFEEST):
    filled, since thin outlines vanish at this size, but with their holes
    (the inside of d and e) left dark. Nesting tells them apart: the blue
    inside an outline that touches the open background is a letter, blue
    inside an outline that does not is a hole. Outlines touching the
    picture's edge (the yellow ring around the logo) are not letters."""
    from scipy import ndimage
    want = np.array([int(colour[i:i + 2], 16) for i in (1, 3, 5)]) / 255
    solid = a[..., 3] > 0.5
    lines = solid & (np.abs(a[..., :3] - want).max(2) < 0.15)
    fill = solid & ~lines
    line_lab, n_lines = ndimage.label(lines)
    fill_lab, _ = ndimage.label(fill)
    background = np.bincount(fill_lab.ravel())[1:].argmax() + 1
    edge = set(np.unique(np.r_[line_lab[0], line_lab[-1], line_lab[:, 0], line_lab[:, -1]]))
    edge |= set(np.unique(line_lab[ndimage.binary_dilation(~solid, iterations=2)]))
    ink = np.zeros(lines.shape, bool)
    for k in range(1, n_lines + 1):
        if k in edge:
            continue
        ring = line_lab == k
        ink |= ring
        around = set(np.unique(fill_lab[ndimage.binary_dilation(ring, iterations=2)])) - {0}
        if background in around:  # a letter's outer outline: fill what it encloses
            inside = ndimage.binary_fill_holes(ring)
            ink |= inside
    # holes: blue that no letter's outer outline reaches stays dark
    for k in range(1, n_lines + 1):
        if k in edge:
            continue
        ring = line_lab == k
        around = set(np.unique(fill_lab[ndimage.binary_dilation(ring, iterations=2)])) - {0}
        if background not in around:
            ink &= ~(ndimage.binary_fill_holes(ring) & ~ring)
    return ink.astype(float)


def globe(rows, spokes=12, rings=(1.0, 0.68, 0.4), hub=0.16, tilt=35, stroke=0.035, flat=0.49):
    """The ALLNET globe: a wire dish, tilted and flattened as in the logo
    (axes measured from allnet.svg), with fewer rings and spokes."""
    a = 1000                                     # semi-major axis, px
    b = a * flat
    img = Image.new("L", (int(2.4 * a), int(1.6 * a)), 0)
    d = ImageDraw.Draw(img)
    cx, cy, t = img.width / 2, img.height / 2, math.radians(tilt)

    def at(u, v):
        x, y = u * a, v * b
        return cx + x * math.cos(t) - y * math.sin(t), cy + x * math.sin(t) + y * math.cos(t)

    width = round(stroke * a)
    for f in rings + (hub,):
        d.line([at(f * math.cos(k * math.pi / 90), f * math.sin(k * math.pi / 90)) for k in range(181)],
               fill=255, width=width, joint="curve")
    for k in range(spokes):
        th = k * 2 * math.pi / spokes
        d.line([at(hub * math.cos(th), hub * math.sin(th)), at(math.cos(th), math.sin(th))], fill=255, width=width)
    return quantise(np.asarray(img).astype(float) / 255, rows, FLOOR)


# name, how
LOGOS = [
    # white brush letters (the SVG has black outlines)
    ("BADGETEAM", lambda: picture("badgeteam.svg", 11, ink="white")),
    # the globe, bigger than the grid: the reel pans along it
    ("ALLNET_GLOBE", lambda: globe(24)),
    # the word; a high floor drops the globe's thin lines behind the A
    ("ALLNET", lambda: picture("allnet.svg", 9, crop=(0.33, 0.25, 1.0, 0.53), floor=0.45)),
    # icon, "Nicolai" and (faint) "Electronics"
    ("NICOLAI", lambda: picture("nicolai.svg", 16)),
    # "de FEEST": the yellow outline letters, filled
    ("DEFEEST", lambda: picture("defeest.svg", 16, ink="#fff56d")),
]


def load(path):
    if path.suffix == ".svg":
        with tempfile.TemporaryDirectory() as d:
            png = Path(d) / "logo.png"
            subprocess.run(["inkscape", str(path), "--export-type=png", f"--export-filename={png}",
                            "--export-width=1600"], check=True, capture_output=True)
            return Image.open(png).convert("RGBA")
    return Image.open(path).convert("RGBA")


def quantise(cover, rows, floor):
    ys, xs = np.nonzero(cover > 0.5)
    cover = cover[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    cols = max(1, round(cover.shape[1] * rows / cover.shape[0]))
    small = np.asarray(Image.fromarray((cover * 255).astype(np.uint8)).resize((cols, rows), Image.BOX)) / 255
    q = np.round(small * LEVELS).astype(int)
    q[small < floor] = 0
    lit = q > 0
    pad = np.pad(lit, 1)
    neighbours = sum(np.roll(np.roll(pad, dy, 0), dx, 1) for dy in (-1, 0, 1) for dx in (-1, 0, 1)
                     if dy or dx)[1:-1, 1:-1]
    q[lit & (neighbours == 0)] = 0
    return q


def packed(q):
    flat = q.flatten()
    flat = np.pad(flat, (0, -len(flat) % 4))
    return [int(sum(int(flat[i + k]) << (2 * k) for k in range(4))) for i in range(0, len(flat), 4)]


def write(logos):
    c = ["/* Generated by tools/sim/logo.py from assets/sponsors; do not edit.",
         " * Row by row, 2 bits a pixel: pixel i = row * w + column is",
         " * data[i / 4] >> (2 * (i % 4)) & 3. */",
         "#pragma once", "#include <stdint.h>", "",
         "typedef struct {", "    uint8_t w, h;", "    const uint8_t *data;", "} logo_t;", ""]
    rs = ["//! Generated by tools/sim/logo.py from assets/sponsors; do not edit.",
          "//! Row by row, 2 bits a pixel: pixel i = row * w + column is",
          "//! data[i / 4] >> (2 * (i % 4)) & 3.", "",
          "pub struct Logo {", "    pub w: u8,", "    pub h: u8,", "    pub data: &'static [u8],", "}", ""]
    for name, q in logos:
        bs = packed(q)
        body = ",\n    ".join(", ".join(f"0x{b:02X}" for b in bs[i:i + 12]) for i in range(0, len(bs), 12))
        h, w = q.shape
        c.append(f"static const uint8_t LOGO_{name}_DATA[{len(bs)}] = {{\n    {body},\n}};")
        c.append(f"static const logo_t LOGO_{name} = {{{w}, {h}, LOGO_{name}_DATA}};")
        rs.append(f"pub const {name}: Logo = Logo {{\n    w: {w},\n    h: {h},\n    data: &[\n    {body},\n    ],\n}};")
    (ROOT / "c" / "src" / "logos.h").write_text("\n".join(c) + "\n")
    (ROOT / "src" / "logos.rs").write_text("\n".join(rs) + "\n")


def preview(logos, out):
    k = 10
    width = max(q.shape[1] for _, q in logos) * k
    img = Image.new("RGB", (width, sum(q.shape[0] * k + 8 for _, q in logos)), (15, 15, 18))
    d = ImageDraw.Draw(img)
    y0 = 0
    for _, q in logos:
        for r in range(q.shape[0]):
            for col in range(q.shape[1]):
                v = int(q[r, col]) * 85
                if v:
                    d.ellipse((col * k + 1, y0 + r * k + 1, col * k + k - 1, y0 + r * k + k - 1),
                              fill=(v, v * 15 // 16, v * 4 // 5))
        y0 += q.shape[0] * k + 8
    img.save(out)


def main(args):
    opts = dict(a[2:].split("=", 1) for a in args if a.startswith("--") and "=" in a)
    logos = []
    for name, make in LOGOS:
        q = make()
        logos.append((name, q))
        print(f"{name}: {q.shape[1]} x {q.shape[0]}, {len(packed(q))} bytes", file=sys.stderr)
    write(logos)
    if "preview" in opts:
        preview(logos, opts["preview"])


if __name__ == "__main__":
    main(sys.argv[1:])
