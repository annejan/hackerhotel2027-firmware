#!/usr/bin/env python3
"""Render frames recorded by the firmware itself (tools/fwsim) on the board.

  (cd tools/fwsim && cargo run -- record ../../rec.bin 300 10:tap3)
  python3 tools/sim/fwgif.py rec.bin out.gif "TITLE" "subtitle" [--buttons=a,b,c,d]
                              [--width=PX] [--skip=FRAMES] [--step=N]

An output ending in .mp4 makes an H.264 video instead of a GIF.

Each recorded frame is the grid (16 x 17), the outline (63 LEDs on the current board), the RGB LEDs
by (arm, rank) (35 x 3), a byte of button state (bits 0-3 held, 4-7 pressed)
and the brightness, as fwsim's dump() writes them.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import games as G  # noqa: E402
import hh2027 as h  # noqa: E402


def main(args):
    opts = dict(a[2:].split("=", 1) for a in args if a.startswith("--"))
    rec, out, title, sub = [a for a in args if not a.startswith("--")]
    pads = {i: p for i, p in enumerate(opts.get("buttons", ",,,").split(",")) if p}
    L = h.Layout()
    R = G.PadRenderer(L)
    GH, GW, NE, NR = L.GH, L.GW, len(L.edge), L.nr
    size = GH * GW + NE + NR * 3 + 2
    raw = np.fromfile(rec, np.uint8)
    step = int(opts.get("step", 1))           # keep every step-th frame (smaller GIF)
    raw = raw[: len(raw) // size * size].reshape(-1, size)[int(opts.get("skip", 0))::step]
    rgb_index = [int(np.nonzero((L.arm == k // 7) & (L.rank == k % 7))[0][0]) for k in range(NR)]
    frames = []
    for row in raw:
        grid = row[: GH * GW].reshape(GH, GW) / 255
        mono = L.grid_to_mono(grid)
        edge = row[GH * GW: GH * GW + NE] / 255
        for j, i in enumerate(L.edge):
            mono[i] = edge[j]
        rgb = np.zeros((NR, 3))
        rgb[rgb_index] = row[GH * GW + NE: GH * GW + NE + NR * 3].reshape(NR, 3) / 255
        state = row[-2]
        lit = {p for p in range(4) if (state >> p) & 1 or (state >> (p + 4)) & 1}
        frames.append(R.frame(mono, rgb, lit, title, sub, pads))
    width = int(opts.get("width", h.S))
    if width != h.S:
        from PIL import Image
        frames = [f.resize((width, round(width * (h.S + h.CAP) / h.S)), Image.LANCZOS) for f in frames]
    h.FPS = 30 // step
    if out.endswith(".mp4"):
        h.save_mp4(frames, Path(out), loops=1)
    else:
        h.save_gif(frames, Path(out))
    print(f"{out}: {len(frames)} frames, {Path(out).stat().st_size // 1024} KiB")


if __name__ == "__main__":
    main(sys.argv[1:])
