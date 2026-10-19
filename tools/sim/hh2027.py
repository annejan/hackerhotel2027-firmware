#!/usr/bin/env python3
"""Animated GIF previews for the Hacker Hotel 2027 starfish badge.

Uses the real board: LED positions come from the KiCad PCB (via pcbnew) and the
background is a KiCad 3D render of the top side, dimmed to look like a dark room.

  208 mono 0402 LEDs   17x16 octagonal grid in the centre, 2 mm pitch
   63 mono 0402 LEDs   following the board outline (71 before 1ce1163)
   35 RGB 1515 LEDs    7 per arm

Usage:
  python3 tools/sim/hh2027.py [--refresh] [--out=DIR] [--width=PX] [--no-mp4] [anim ...]

Writes GIFs and MP4s to previews/. The LED positions and the board render are
cached next to this script, so a normal run needs no KiCad. --refresh
re-extracts them from a hackerhotel2027-hardware checkout next to this repo
(or $HH27_HW), and needs kicad-cli and the pcbnew Python module.
"""
import colorsys
import json
import math
import os
import random
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as nd

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
LED_JSON = HERE / "hh2027_leds.json"
RENDER = HERE / "hh2027_render.png"
OUT = REPO / "previews"


def find_pcb():
    """hackerhotel2027-hardware checkout: $HH27_HW, else a sibling of this repo."""
    name = "hackerhotel2027-hardware"
    cands = [Path(os.environ["HH27_HW"])] if "HH27_HW" in os.environ else []
    cands += [REPO.parent / name, REPO.parent.parent / name]
    for d in cands:
        if (d / "hackerhotel2027.kicad_pcb").exists():
            return d / "hackerhotel2027.kicad_pcb"
    sys.exit(f"can't find {name}; clone it next to this repo or set HH27_HW")

RENDER_PX = 1376             # asked of kicad-cli; it may deliver a different size
S = 480                      # board image size in the GIF
CAP = 60                     # caption band
FPS = 20
AMBIENT = 0.22               # how much of the 3D render shows through ("dark room")
MONO_COL = np.array([1.0, .96, .88])   # colour of the 0402s: unknown yet, warm white


# --------------------------------------------------------- board extraction ---
def refresh():
    import pcbnew
    pcb = find_pcb()
    b = pcbnew.LoadBoard(str(pcb))
    mm = pcbnew.ToMM
    bb = b.GetBoardEdgesBoundingBox()
    leds = []
    for fp in b.GetFootprints():
        ref, lib = fp.GetReference(), str(fp.GetFPID().GetLibItemName())
        if ref.startswith("D") and ("0402" in lib or "B1515" in lib):
            p = fp.GetPosition()
            leds.append(dict(ref=ref, rgb="B1515" in lib, x=mm(p.x), y=mm(p.y)))
    # the push buttons, for the emulator (all four on the back since 0986add)
    buttons = []
    for ref in ("SW1", "SW2", "SW3", "SW4"):
        fp = b.FindFootprintByReference(ref)
        if fp:
            p = fp.GetPosition()
            buttons.append(dict(ref=ref, x=mm(p.x), y=mm(p.y), back=fp.IsFlipped()))
    LED_JSON.write_text(json.dumps(dict(
        bbox=[mm(bb.GetX()), mm(bb.GetY()), mm(bb.GetWidth()), mm(bb.GetHeight())],
        leds=leds, buttons=buttons), indent=1))
    subprocess.run(["kicad-cli", "pcb", "render", "--side", "top", "--quality", "high",
                    "--width", str(RENDER_PX), "--height", str(RENDER_PX),
                    "--background", "opaque", "--use-board-stackup-colors",
                    "-o", str(RENDER), str(pcb)], check=True)


class Layout:
    def __init__(self):
        d = json.loads(LED_JSON.read_text())
        k, ox, oy = self.calibrate(d)
        # scale from the render's real size: kicad-cli can return a slightly
        # different size than asked (1344 for 1376 here), which would pull
        # every LED towards the top left
        px = Image.open(RENDER).size[0]
        self.to_px = lambda x, y: ((ox + x * k) * S / px, (oy + y * k) * S / px)

        mono = [l for l in d["leds"] if not l["rgb"]]
        rgb = [l for l in d["leds"] if l["rgb"]]
        self.center = np.array([np.mean([l["x"] for l in rgb]), np.mean([l["y"] for l in rgb])])

        # mono: split into the centre grid and the outline
        self.grid_rc, self.edge = {}, []
        mx = np.array([[l["x"], l["y"]] for l in mono])
        # grid is on a 2 mm lattice: find its phase from the most common x/y mod 2
        def phase(v):
            vals, cnt = np.unique(np.round(v % 2, 1), return_counts=True)
            return vals[cnt.argmax()]
        px_, py_ = phase(mx[:, 0]), phase(mx[:, 1])
        for i, (x, y) in enumerate(mx):
            c, r = (x - px_) / 2, (y - py_) / 2
            if abs(x) < 18 and abs(y) < 18 and abs(c - round(c)) < .1 and abs(r - round(r)) < .1:
                self.grid_rc[i] = (int(round(r)), int(round(c)))
            else:
                self.edge.append(i)
        r0 = min(r for r, c in self.grid_rc.values())
        c0 = min(c for r, c in self.grid_rc.values())
        self.grid_rc = {i: (r - r0, c - c0) for i, (r, c) in self.grid_rc.items()}
        self.GW = max(c for r, c in self.grid_rc.values()) + 1
        self.GH = max(r for r, c in self.grid_rc.values()) + 1
        self.gmask = np.zeros((self.GH, self.GW), bool)
        for r, c in self.grid_rc.values():
            self.gmask[r, c] = True
        # clockwise from the top, as tools/gen_layout.py orders the firmware's
        # EDGE: by angle in 1/256 turn, ties in board order
        rel = mx[self.edge] - self.center
        turn = np.round(np.arctan2(rel[:, 0], -rel[:, 1]) % (2 * np.pi) / (2 * np.pi) * 256) % 256
        self.edge = [self.edge[j] for j in np.argsort(turn, kind="stable")]
        self.mono_mm = mx

        # rgb: group by arm (angle sector), order centre -> tip
        rx = np.array([[l["x"], l["y"]] for l in rgb])
        rr = rx - self.center
        ang = (np.degrees(np.arctan2(rr[:, 0], -rr[:, 1])) + 36) % 360   # 0 = top arm
        self.arm = (ang // 72).astype(int)
        rad = np.hypot(rr[:, 0], rr[:, 1])
        self.rank = np.zeros(len(rgb), int)
        for a_ in range(5):
            idx = np.nonzero(self.arm == a_)[0]
            self.rank[idx[np.argsort(rad[idx])]] = np.arange(len(idx))
        self.rgb_mm = rx
        print(f"layout: {len(self.grid_rc)} grid ({self.GW}x{self.GH}), {len(self.edge)} edge, "
              f"{len(rgb)} rgb, per arm {np.bincount(self.arm).tolist()}")

        self.mono_px = np.array([self.to_px(*p) for p in mx])
        self.rgb_px = np.array([self.to_px(*p) for p in rx])

        # polar coords for everything (mm, angle 0 = up, clockwise)
        def polar(p):
            v = p - self.center
            return np.hypot(v[:, 0], v[:, 1]), np.arctan2(v[:, 0], -v[:, 1])
        self.mr, self.mth = polar(mx)
        self.rr, self.rth = polar(rx)
        self.nm, self.nr = len(mx), len(rx)

    @staticmethod
    def calibrate(d):
        """Render px = offset + mm * k. A first guess comes from the board's
        bounding box in the render (the background is tinted blue); it is
        then fitted to the RGB LEDs, which show as bright white squares."""
        bx, by, bw, bh = d["bbox"]
        a = np.asarray(Image.open(RENDER).convert("RGB")).astype(int)
        ys, xs = np.nonzero(np.abs(a[:, :, 2] - a[:, :, 0]) < 14)
        k = (xs.max() - xs.min()) / bw
        ox, oy = xs.min() - bx * k, ys.min() - by * k
        gray = a.mean(2)
        rgb = [(l["x"], l["y"]) for l in d["leds"] if l["rgb"]]
        for _ in range(3):
            mm, px = [], []
            for x, y in rgb:
                cx, cy = int(ox + x * k), int(oy + y * k)
                win = gray[cy - 30:cy + 31, cx - 30:cx + 31]
                wy, wx = np.nonzero(win > 245)
                if len(wx) > 30:
                    mm.append((x, y))
                    px.append((cx - 30 + wx.mean(), cy - 30 + wy.mean()))
            if len(mm) < 10:
                break
            mm, px = np.array(mm), np.array(px)
            # least squares for one scale and two offsets
            A = np.zeros((2 * len(mm), 3))
            A[0::2, 0], A[0::2, 1] = mm[:, 0], 1
            A[1::2, 0], A[1::2, 2] = mm[:, 1], 1
            (k, ox, oy), *_ = np.linalg.lstsq(A, px.ravel(), rcond=None)
        return k, ox, oy

    def grid_to_mono(self, img):
        """img: (GH, GW) intensities -> mono array (edge LEDs left at 0)."""
        m = np.zeros(self.nm)
        for i, (r, c) in self.grid_rc.items():
            m[i] = img[r, c]
        return m


# ------------------------------------------------------------- renderer ---
class Renderer:
    def __init__(self, L):
        self.L = L
        img = Image.open(RENDER).convert("RGB").resize((S, S), Image.LANCZOS)
        a = np.asarray(img).astype(np.float32) / 255
        a = nd.gaussian_filter(a, (.8, .8, 0))      # soften render noise: GIFs compress far better
        self.albedo = a.mean(2, keepdims=True)
        self.base = a * AMBIENT * np.array([.85, .9, 1.1])   # slightly cool night light
        self.f_title = ImageFont.truetype("/usr/share/fonts/truetype/DejaVuSans-Bold.ttf", 20)
        self.f_sub = ImageFont.truetype("/usr/share/fonts/truetype/DejaVuSans.ttf", 13)

    @staticmethod
    def splat(px, vals, ch):
        im = np.zeros((S, S, ch), np.float32)
        for (x, y), v in zip(px, vals):
            if np.any(v > 0.003):
                xi, yi = int(x), int(y)
                fx, fy = x - xi, y - yi
                for dx, dy, w in ((0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)),
                                  (0, 1, (1 - fx) * fy), (1, 1, fx * fy)):
                    if 0 <= yi + dy < S and 0 <= xi + dx < S:
                        im[yi + dy, xi + dx] += v * w
        return im

    @staticmethod
    def glow(im, sig):
        out = np.empty_like(im)
        for c in range(im.shape[2]):
            out[:, :, c] = nd.gaussian_filter(im[:, :, c], sig, truncate=3)
        return out * (2 * math.pi * sig * sig)

    def render(self, mono, rgb, title, sub):
        mono = np.clip(mono, 0, 1)
        rgb = np.clip(rgb, 0, 1)
        im_m = self.splat(self.L.mono_px, mono[:, None] * MONO_COL[None, :], 3)
        im_r = self.splat(self.L.rgb_px, rgb, 3)
        core = self.glow(im_m, 1.3) * 1.3 + self.glow(im_r, 2.6) * 1.8
        hot = self.glow(im_m.max(2, keepdims=True), .8) * .5 + \
            self.glow(im_r.max(2, keepdims=True), 1.4) * .8
        halo = self.glow(im_m, 5.5) * .22 + self.glow(im_r, 13) * .42
        img = self.base + core + hot + halo * (.3 + .9 * self.albedo)
        img = (np.clip(img, 0, 1) * 255).astype(np.uint8)

        out = Image.new("RGB", (S, S + CAP), (17, 19, 24))
        out.paste(Image.fromarray(img), (0, 0))
        dr = ImageDraw.Draw(out)
        dr.line([0, S, S, S], fill=(40, 43, 52))
        dr.text((16, S + 9), title, font=self.f_title, fill=(242, 239, 230))
        dr.text((16, S + 36), sub, font=self.f_sub, fill=(154, 160, 171))
        return out


# ------------------------------------------------------------ helpers ---
def hsv(h, s=1.0, v=1.0):
    return np.array(colorsys.hsv_to_rgb(h % 1.0, s, v))


def hsv_arr(h, s=1.0, v=1.0):
    h = np.asarray(h) % 1.0
    s = np.broadcast_to(s, h.shape)
    v = np.broadcast_to(v, h.shape)
    return np.array([colorsys.hsv_to_rgb(*x) for x in zip(h, s, v)]).reshape(h.shape + (3,))


FONT = {  # 5x7, rows top -> bottom
    "H": "10001 10001 10001 11111 10001 10001 10001",
    "A": "01110 10001 10001 11111 10001 10001 10001",
    "C": "01110 10001 10000 10000 10000 10001 01110",
    "K": "10001 10010 10100 11000 10100 10010 10001",
    "E": "11111 10000 10000 11110 10000 10000 11111",
    "R": "11110 10001 10001 11110 10100 10010 10001",
    "O": "01110 10001 10001 10001 10001 10001 01110",
    "T": "11111 00100 00100 00100 00100 00100 00100",
    "L": "10000 10000 10000 10000 10000 10000 11111",
    "2": "01110 10001 00001 00110 01000 10000 11111",
    "0": "01110 10011 10101 10101 11001 10001 01110",
    "7": "11111 00001 00010 00100 01000 01000 01000",
    "*": "00000 00100 10101 01110 10101 00100 00000",
    " ": "00000 00000 00000 00000 00000 00000 00000",
}


def text_bitmap(s):
    cols = []
    for ch in s:
        rows = FONT[ch].split()
        for c in range(5):
            cols.append([int(rows[r][c]) for r in range(7)])
        cols.append([0] * 7)
    return np.array(cols, float).T          # (7, W)


# ---------------------------------------------------------- animations ---
# Each returns a list of (mono[279], rgb[35,3]) frames.

def ripple(L, T=60):
    fr = []
    for t in range(T):
        ph = t / 30
        m = np.maximum(0, np.cos(2 * math.pi * (L.mr / 11 - ph))) ** 4
        v = np.maximum(0, np.cos(2 * math.pi * (L.rr / 11 - ph))) ** 2
        rgb = hsv_arr(L.rr / 45 - t / T) * (.15 + .85 * v)[:, None]
        fr.append((m, rgb))
    return fr


def radar(L, T=80):
    rnd = random.Random(3)
    blips = [rnd.randrange(L.nr) for _ in range(6)]
    fr = []
    for t in range(T):
        phi = 2 * math.pi * t / T
        d = (phi - L.mth) % (2 * math.pi)
        m = np.exp(-d / .55) * (L.mr < 60)
        dr = (phi - L.rth) % (2 * math.pi)
        rgb = np.exp(-dr / .7)[:, None] * np.array([.1, 1, .3]) * .6
        for b in blips:
            rgb[b] = np.maximum(rgb[b], np.array([1, .15, .05]) * math.exp(-dr[b] / 1.2))
        fr.append((m, rgb))
    return fr


def scroller(L, msg="HACKER HOTEL 2027 * "):
    bm = text_bitmap(msg)
    W = bm.shape[1]
    T = W * 2
    fr = []
    for t in range(T):
        off = t // 2
        img = np.zeros((L.GH, L.GW))
        for c in range(L.GW):
            img[4:11, c] = bm[:, (off + c) % W]
        m = L.grid_to_mono(img)
        for j, i in enumerate(L.edge):                 # marquee round the edge
            m[i] = .55 if (j - t // 2) % 4 == 0 else .04
        rgb = hsv_arr(L.rank / 7 + L.arm / 5 - t / 40, 1, 1) * .8
        fr.append((m, rgb))
    return fr


def plasma(L, T=100):
    fr = []
    for t in range(T):
        w = 2 * math.pi * t / T

        def f(p, r):
            x, y = p[:, 0], p[:, 1]
            return (np.sin(x / 5 + w) + np.sin(y / 6.5 - 2 * w) +
                    np.sin((x + y) / 9 + w) + np.sin(r / 4 - 3 * w)) / 4
        vm = f(L.mono_mm, L.mr)
        vr = f(L.rgb_mm, L.rr)
        m = ((vm + 1) / 2) ** 2.2
        rgb = hsv_arr(vr * .6 + t / T) * (.25 + .75 * ((vr + 1) / 2))[:, None]
        fr.append((m, rgb))
    return fr


def heartbeat(L, T=44):
    def beat(t):
        t %= T
        return math.exp(-((t - 4) / 2.2) ** 2) + .7 * math.exp(-((t - 11) / 2.2) ** 2)
    fr = []
    for t in range(T):
        m = np.array([beat(t - r / 6) for r in L.mr]) * .9
        v = np.array([beat(t - r / 6) for r in L.rr])
        rgb = (np.array([1, .05, .18]) * (.06 + v[:, None]))
        fr.append((m, rgb))
    return fr


def life(L, T=220, step=4):
    rnd = np.random.default_rng(11)
    g = (rnd.random((L.GH, L.GW)) < .38) & L.gmask
    prev = g.copy()
    hist = []
    fr = []
    births = 0
    for t in range(T):
        if t % step == 0 and t:
            prev = g.copy()
            n = sum(np.roll(np.roll(g, dy, 0), dx, 1) for dy in (-1, 0, 1) for dx in (-1, 0, 1)
                    if dy or dx)
            g = ((n == 3) | (g & (n == 2))) & L.gmask
            births = int((g & ~prev).sum())
            hist.append(g.tobytes())
            if g.sum() < 8 or hist.count(g.tobytes()) > 2:
                g = (rnd.random((L.GH, L.GW)) < .38) & L.gmask
                hist = []
        a = (t % step) / step
        img = prev * (1 - a) + g * a if t else g.astype(float)
        m = L.grid_to_mono(img * .9)
        for j, i in enumerate(L.edge):
            m[i] = .08
        pop = g.sum() / L.gmask.sum()
        rgb = hsv_arr(.33 + pop * 1.5 + L.rank * .03, 1, 1) * \
            (.15 + .6 * math.exp(-(t % step) / 2) * min(1, births / 12))
        fr.append((m, rgb))
    return fr


def fire(L, T=140):
    rnd = np.random.default_rng(5)
    H, W = L.GH + 2, L.GW
    heat = np.zeros((H, W))
    # edge LEDs: map height on board to a flicker level
    ey = L.mono_mm[L.edge][:, 1]
    eh = (ey - ey.min()) / (ey.max() - ey.min())
    fr = []
    for t in range(T + 40):
        heat[-1] = rnd.random(W) * .6 + .5
        new = heat.copy()
        for y in range(H - 1):
            src = heat[y + 1]
            sh = rnd.integers(-1, 2, W)
            new[y] = np.clip(src[(np.arange(W) + sh) % W] - rnd.random(W) * .13 - .02, 0, 1)
        heat = new
        if t < 40:
            continue
        img = heat[:L.GH] ** 1.6
        m = L.grid_to_mono(img)
        for j, i in enumerate(L.edge):
            m[i] = max(0, eh[j] - .35) * (.4 + .6 * rnd.random()) * .8
        lvl = heat[:L.GH].mean(1)
        ry = np.clip(((L.rgb_mm[:, 1] - L.center[1]) / 45 + .5), 0, 1)
        v = np.clip(ry * 1.3 - .2 + rnd.random(L.nr) * .3, 0, 1) * (.5 + lvl.mean())
        rgb = np.stack([np.ones(L.nr), .18 + .4 * v, .02 * np.ones(L.nr)], 1) * v[:, None]
        fr.append((m, rgb))
    return fr


def rain(L, T=120):
    rnd = random.Random(9)
    drops = [[rnd.uniform(-16, 16), rnd.uniform(.25, .6), c] for c in range(L.GW)]
    drops += [[rnd.uniform(-30, 0), rnd.uniform(.25, .6), rnd.randrange(L.GW)] for _ in range(8)]
    fr = []
    for t in range(T):
        img = np.zeros((L.GH, L.GW))
        for d in drops:
            d[0] += d[1]
            if d[0] - 8 > L.GH:
                d[0], d[1] = rnd.uniform(-10, -1), rnd.uniform(.25, .6)
            head = int(d[0])
            for k in range(8):
                y = head - k
                if 0 <= y < L.GH:
                    img[y, d[2]] = max(img[y, d[2]], 1.0 if k == 0 else .55 * (.72 ** k))
        m = L.grid_to_mono(img)
        rgb = np.zeros((L.nr, 3))
        for i in range(L.nr):
            if rnd.random() < .06:
                rgb[i] = [.2, 1, .3]
        if fr:
            rgb = np.maximum(rgb, fr[-1][1] * .7)
        fr.append((m, rgb))
    return fr


def charging(L, T=120):
    # battery outline on the grid, filling up
    fr = []
    for t in range(T):
        lvl = min(1, t / (T * .8))
        img = np.zeros((L.GH, L.GW))
        r0, r1, c0, c1 = 4, 11, 2, 13
        img[r0, c0:c1 + 1] = img[r1, c0:c1 + 1] = 1
        img[r0:r1 + 1, c0] = img[r0:r1 + 1, c1] = 1
        img[6:10, c1 + 1] = 1
        cells = 10
        n = int(lvl * cells + .999) if t < T * .8 else cells
        for c in range(n):
            blink = 1 if (c < n - 1 or t >= T * .8) else .3 + .7 * (t // 4 % 2)
            img[r0 + 2:r1 - 1, c0 + 1 + c] = .7 * blink
        if t >= T * .8:
            img *= .6 + .4 * math.sin((t - T * .8) / 2)
        m = L.grid_to_mono(img)
        for j, i in enumerate(L.edge):
            m[i] = .25 * (.5 + .5 * math.sin(t / 6 + j / len(L.edge) * 2 * math.pi))
        on = L.rank < lvl * 7
        rgb = np.where(on[:, None], hsv_arr(.33 * lvl * np.ones(L.nr)), 0) * .8
        fr.append((m, rgb))
    return fr


def twinkle(L, T=120):
    rnd = np.random.default_rng(2)
    f = rnd.integers(1, 4, L.nm)          # whole cycles per loop keeps it seamless
    p = rnd.random(L.nm)
    fr = []
    for t in range(T):
        m = np.maximum(0, np.sin(2 * math.pi * (f * t / T + p))) ** 12
        rgb = np.zeros((L.nr, 3))
        for a in range(5):
            pos = (t / T * 2 + a / 5) % 1 * 11 - 2        # comet runs centre -> tip
            for i in np.nonzero(L.arm == a)[0]:
                d = pos - L.rank[i]
                if 0 <= d < 4:
                    rgb[i] = hsv(.55 + a * .1, .6 + .1 * d, 1) * (.75 ** (d * 2))
        fr.append((m * .9, rgb))
    return fr


def galaxy(L, T=90):
    fr = []
    for t in range(T):
        w = 2 * math.pi * t / T
        m = (np.maximum(0, np.cos(5 * L.mth + L.mr / 4 - w)) ** 3) * np.clip(1.1 - L.mr / 60, .2, 1)
        v = np.maximum(0, np.cos(5 * L.rth + L.rr / 4 - w))
        rgb = hsv_arr(L.rth / (2 * math.pi) + t / T) * (.15 + .85 * v ** 2)[:, None]
        fr.append((m, rgb))
    return fr


ANIMS = {
    "ripple": ("RIPPLE", "radial waves from the centre", ripple),
    "radar": ("RADAR", "sweep with afterglow, red blips on the arms", radar),
    "scroller": ("SCROLLER", "17x16 grid as display · edge marquee", scroller),
    "plasma": ("PLASMA", "sine plasma, grayscale on mono, hue on RGB", plasma),
    "heartbeat": ("HEARTBEAT", "double pulse travelling outward", heartbeat),
    "life": ("GAME OF LIFE", "Conway on the grid, arms react to births", life),
    "fire": ("FIRE", "doom-fire on the grid, embers on the arms", fire),
    "rain": ("DIGITAL RAIN", "falling trails on the grid", rain),
    "charging": ("CHARGING", "battery gauge (CHRG pin) · arms fill green", charging),
    "twinkle": ("TWINKLE", "random mono sparkle, comets up the arms", twinkle),
    "galaxy": ("GALAXY", "five-armed spiral, rotating", galaxy),
}


def save_gif(frames, path):
    # ffmpeg: one shared palette, and unchanged pixels become transparent
    # (gifflags transdiff), which keeps the mostly-static board cheap
    w, h = frames[0].size
    flt = ("split[a][b];[a]palettegen=max_colors=255:stats_mode=full[p];"
           "[b][p]paletteuse=dither=none:diff_mode=rectangle")
    proc = subprocess.Popen(
        ["ffmpeg", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24",
         "-s", f"{w}x{h}", "-r", str(FPS), "-i", "-", "-filter_complex", flt,
         "-loop", "0", str(path)], stdin=subprocess.PIPE)
    for f in frames:
        proc.stdin.write(f.tobytes())
    proc.stdin.close()
    if proc.wait():
        raise RuntimeError("ffmpeg failed")


def save_mp4(frames, path, loops=3):
    w, h = frames[0].size
    proc = subprocess.Popen(
        ["ffmpeg", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24",
         "-s", f"{w}x{h}", "-r", str(FPS), "-i", "-", "-c:v", "libx264", "-crf", "20",
         "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(path)], stdin=subprocess.PIPE)
    for _ in range(loops):
        for f in frames:
            proc.stdin.write(f.tobytes())
    proc.stdin.close()
    if proc.wait():
        raise RuntimeError("ffmpeg failed")


def main(args):
    opts = dict(a[2:].split("=", 1) if "=" in a else (a[2:], "") for a in args if a.startswith("--"))
    if "refresh" in opts or not LED_JSON.exists() or not RENDER.exists():
        refresh()
    out = Path(opts.get("out", OUT))
    width = int(opts.get("width", S))         # frames are rendered at S, then scaled
    names = [a for a in args if not a.startswith("--")] or list(ANIMS)
    L = Layout()
    R = Renderer(L)
    out.mkdir(parents=True, exist_ok=True)
    for name in names:
        title, sub, fn = ANIMS[name]
        frames = [R.render(m, c, title, sub) for m, c in fn(L)]
        if width != S:
            size = (width, round(width * (S + CAP) / S))
            frames = [f.resize(size, Image.LANCZOS) for f in frames]
        path = out / f"{name}.gif"
        save_gif(frames, path)
        msg = f"{path.name}: {len(frames)} frames, {path.stat().st_size // 1024} KiB"
        if "no-mp4" not in opts:
            save_mp4(frames, path.with_suffix(".mp4"))
            msg += f" (mp4 {path.with_suffix('.mp4').stat().st_size // 1024} KiB)"
        print(msg)


if __name__ == "__main__":
    main(sys.argv[1:])
