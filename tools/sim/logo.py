#!/usr/bin/env python3
"""Turn a picture into a sponsor logo for the badge's grid (sponsors.c / .rs).

  python3 tools/sim/logo.py LOGO.png [--threshold=128] [--invert] [--size=ROWSxCOLS]

Scales the picture to fit the grid (16 rows x 17 columns, keeping its
aspect ratio), makes it one bit a pixel and prints a preview and the rows
for C and Rust: bit c of a row is column c, as MASK. The octagon's cut-off
corners have no LEDs; the preview marks what falls there with x.
Transparent pixels are background. Without transparency, the colour at the
picture's border is taken as background (--invert swaps).

Small pictures work best: a 16 x 17 mark or a few letters, no fine lines.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import hh2027 as h  # noqa: E402


def main(args):
    opts = dict(a[2:].split("=", 1) if "=" in a else (a[2:], "1") for a in args if a.startswith("--"))
    files = [a for a in args if not a.startswith("--")]
    if len(files) != 1:
        sys.exit(__doc__)
    L = h.Layout()
    gh, gw = L.GH, L.GW
    rows, cols = (int(v) for v in opts.get("size", f"{gh}x{gw}").split("x"))
    img = Image.open(files[0]).convert("RGBA")
    img.thumbnail((cols, rows), Image.LANCZOS)
    a = np.asarray(img).astype(float) / 255
    alpha = a[..., 3]
    lum = a[..., :3] @ [0.299, 0.587, 0.114]
    if alpha.min() < 0.5:
        on = alpha > 0.5
    else:
        border = np.concatenate([lum[0], lum[-1], lum[:, 0], lum[:, -1]]).mean()
        on = np.abs(lum - border) > int(opts.get("threshold", 128)) / 255
    if "invert" in opts:
        on = ~on
    bits = np.zeros((gh, gw), bool)
    r0, c0 = (gh - on.shape[0]) // 2, (gw - on.shape[1]) // 2
    bits[r0:r0 + on.shape[0], c0:c0 + on.shape[1]] = on
    print("preview (# lit, x outside the octagon):", file=sys.stderr)
    for r in range(gh):
        print("  " + "".join("#" if bits[r, c] and L.gmask[r, c] else "x" if bits[r, c] else
                             "." if L.gmask[r, c] else " " for c in range(gw)), file=sys.stderr)
    words = [sum(1 << c for c in range(gw) if bits[r, c] and L.gmask[r, c]) for r in range(gh)]
    name = Path(files[0]).stem.upper().replace("-", "_").replace(".", "_")
    hexes = ", ".join(f"0x{w:05X}" for w in words)
    print(f"// C, sponsors.c\nstatic const uint32_t LOGO_{name}[GRID_H] = {{{hexes}}};")
    print(f"// Rust, sponsors.rs\nconst LOGO_{name}: [u32; GRID_H] = [{hexes}];")


if __name__ == "__main__":
    main(sys.argv[1:])
