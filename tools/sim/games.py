#!/usr/bin/env python3
"""Game mock-ups for the Hacker Hotel 2027 starfish badge.

Each game is played by a small scripted AI and rendered with the same board
renderer as hh2027.py. The caption shows the four touch pads (T1..T4), lit
while the AI "presses" them.

  python3 tools/sim/games.py [--out=DIR] [--width=PX] [--no-mp4] [game ...]

The playfield is the 17 x 16 octagonal grid; the 71 outline LEDs and the
35 RGB LEDs (7 per arm) carry score, lives and effects.
"""
import random
import sys
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent))
import hh2027 as h  # noqa: E402

PRESS_FRAMES = 4      # how long a press stays lit in the caption

DIGITS = {  # 3x5, rows top -> bottom
    "0": "111 101 101 101 111", "1": "010 110 010 010 111", "2": "111 001 111 100 111",
    "3": "111 001 111 001 111", "4": "101 101 111 001 001", "5": "111 100 111 001 111",
    "6": "111 100 111 101 111", "7": "111 001 010 010 010", "8": "111 101 111 101 111",
    "9": "111 101 111 001 111",
}


class Game:
    """Frame builder shared by all games: grid, outline, arms and pad presses."""

    def __init__(self, L, pads):
        self.L = L
        self.pads = pads              # role label per pad, None = unused
        self.frames = []
        self.lit = {}                 # pad -> frames left
        self.new()

    def new(self):
        self.grid = np.zeros((self.L.GH, self.L.GW))
        self.edge = np.zeros(len(self.L.edge))
        self.rgb = np.zeros((self.L.nr, 3))

    def press(self, pad):
        self.lit[pad] = PRESS_FRAMES

    def arm(self, a, n, col, v=1.0):
        """Light the first n LEDs (centre outward) of arm a."""
        for i in np.nonzero(self.L.arm == a)[0]:
            if self.L.rank[i] < n:
                self.rgb[i] = np.asarray(col, float) * v

    def arms_all(self, col, v=1.0):
        self.rgb[:] = np.asarray(col, float) * v

    def rainbow(self, t, v=1.0):
        self.rgb = h.hsv_arr(self.L.arm / 5 + self.L.rank / 14 - t / 20) * v

    def digits(self, s, row=5, v=1.0):
        """Small 3x5 number, centred on the grid."""
        w = len(s) * 4 - 1
        c0 = (self.L.GW - w) // 2
        for k, ch in enumerate(s):
            rows = DIGITS[ch].split()
            for r in range(5):
                for c in range(3):
                    if rows[r][c] == "1":
                        self.grid[row + r, c0 + k * 4 + c] = v

    def emit(self):
        mono = self.L.grid_to_mono(np.clip(self.grid, 0, 1) * self.L.gmask)
        for j, i in enumerate(self.L.edge):
            mono[i] = self.edge[j]
        self.frames.append((mono, self.rgb.copy(), set(self.lit)))
        self.lit = {p: n - 1 for p, n in self.lit.items() if n > 1}
        self.new()

    def score_screen(self, score, n=30):
        for t in range(n):
            self.digits(str(score), v=1.0 if t % 10 < 7 else .3)
            self.rainbow(t, .8)
            self.edge[:] = .25
            self.emit()

    def walls(self, v=.06):
        """Faintly show the octagon so the playfield edge is visible."""
        self.grid[self.L.gmask] = np.maximum(self.grid[self.L.gmask], v)


def inside(L, r, c):
    return 0 <= r < L.GH and 0 <= c < L.GW and L.gmask[r, c]


# ------------------------------------------------------------------ snake ---
def snake(L):
    g = Game(L, {0: "◀ turn", 3: "turn ▶"})
    rnd = random.Random(4)
    body = deque([(8, 6), (8, 5), (8, 4)])
    d = (0, 1)
    free = [(r, c) for r in range(L.GH) for c in range(L.GW) if L.gmask[r, c]]

    def new_food():
        return rnd.choice([p for p in free if p not in body])

    food = new_food()
    score, t, dead = 0, 0, False
    flash = 0
    while not dead:
        if t % 3 == 0:
            # BFS from head to food, avoiding the body
            head = body[0]
            want = None
            if t < 330:
                prev = {head: None}
                q = deque([head])
                blocked = set(list(body)[:-1])
                while q:
                    p = q.popleft()
                    if p == food:
                        break
                    for dr, dc in ((0, 1), (1, 0), (0, -1), (-1, 0)):
                        n = (p[0] + dr, p[1] + dc)
                        if n not in prev and inside(L, *n) and n not in blocked:
                            prev[n] = p
                            q.append(n)
                if food in prev:
                    p = food
                    while prev[p] != head:
                        p = prev[p]
                    want = (p[0] - head[0], p[1] - head[1])
                else:  # no path: any safe neighbour
                    for dd in (d, (-d[1], d[0]), (d[1], -d[0])):
                        n = (head[0] + dd[0], head[1] + dd[1])
                        if inside(L, *n) and n not in blocked:
                            want = dd
                            break
            if want and want != d:
                if want == (-d[1], d[0]):
                    g.press(0)
                elif want == (d[1], -d[0]):
                    g.press(3)
                else:                      # reverse: turn twice over two steps
                    g.press(0)
                    want = (-d[1], d[0])
                d = want
            n = (body[0][0] + d[0], body[0][1] + d[1])
            if not inside(L, *n) or n in list(body)[:-1]:
                dead = True
            else:
                body.appendleft(n)
                if n == food:
                    score += 1
                    flash = 8
                    food = new_food()
                else:
                    body.pop()
        g.walls()
        for k, (r, c) in enumerate(body):
            g.grid[r, c] = 1.0 if k == 0 else max(.35, .8 - k * .02)
        if t % 6 < 4:
            g.grid[food] = .9
        for a in range(5):
            g.arm(a, min(7, score), (.1, 1, .25), .7)
        if flash:
            g.arms_all((.2, 1, .3), flash / 8)
            g.edge[:] = flash / 10
            flash -= 1
        g.emit()
        t += 1
    for k in range(18):                    # crash
        if k % 6 < 3:
            for r, c in body:
                g.grid[r, c] = 1
        g.arms_all((1, .05, 0), 1 - k / 18)
        g.edge[:] = (1 - k / 18) * .6
        g.emit()
    g.score_screen(score)
    return g


# ----------------------------------------------------------------- tetris ---
SHAPES = {
    "I": [(0, 0), (0, 1), (0, 2), (0, 3)], "O": [(0, 0), (0, 1), (1, 0), (1, 1)],
    "T": [(0, 0), (0, 1), (0, 2), (1, 1)], "S": [(0, 1), (0, 2), (1, 0), (1, 1)],
    "Z": [(0, 0), (0, 1), (1, 1), (1, 2)], "J": [(0, 0), (1, 0), (1, 1), (1, 2)],
    "L": [(0, 2), (1, 0), (1, 1), (1, 2)],
}
HUE = {"I": .5, "O": .15, "T": .8, "S": .33, "Z": 0, "J": .62, "L": .08}


def rotations(cells):
    out, cur = [], cells
    for _ in range(4):
        mr, mc = min(r for r, c in cur), min(c for r, c in cur)
        norm = tuple(sorted((r - mr, c - mc) for r, c in cur))
        if norm not in out:
            out.append(norm)
        cur = [(c, -r) for r, c in cur]
    return out


def tetris(L):
    W, H, R0, C0 = 9, 14, 1, 4          # well: rows 1..14, cols 4..12
    g = Game(L, {0: "◀", 1: "rot", 2: "drop", 3: "▶"})
    rnd = random.Random(7)
    field = np.zeros((H, W), int)       # 0 empty, else piece index + 1
    bag = []

    def next_piece():
        if not bag:
            bag.extend(rnd.sample(list(SHAPES), 7))
        return bag.pop()

    def fits(cells, r, c):
        return all(0 <= r + dr < H and 0 <= c + dc < W and not field[r + dr, c + dc]
                   for dr, dc in cells)

    def drop_row(cells, c):
        r = 0
        while fits(cells, r + 1, c):
            r += 1
        return r

    def evaluate(cells, c):
        if not fits(cells, 0, c):
            return -1e9
        r = drop_row(cells, c)
        f = field.copy()
        for dr, dc in cells:
            f[r + dr, c + dc] = 1
        full = [i for i in range(H) if f[i].all()]
        f = np.delete(f, full, 0)
        heights = [H - next((i for i in range(len(f)) if f[i, x]), len(f)) - len(full)
                   for x in range(W)]
        holes = sum(1 for x in range(W) for i in range(len(f))
                    if not f[i, x] and f[:i, x].any())
        bump = sum(abs(a - b) for a, b in zip(heights, heights[1:]))
        return -.51 * sum(heights) + .76 * len(full) - .36 * holes - .18 * bump

    lines, cur, nxt = 0, next_piece(), next_piece()
    t = 0

    def draw(active=None, blink=()):
        g.walls()
        for r in range(H):
            for c in range(W):
                if field[r, c]:
                    g.grid[R0 + r, C0 + c] = .25 if r in blink else .7
        for c in range(L.GW):           # well walls
            for r in range(R0, R0 + H):
                if c in (C0 - 1, C0 + W) and L.gmask[r, c]:
                    g.grid[r, c] = .15
        if active:
            cells, r, c = active
            for dr, dc in cells:
                g.grid[R0 + r + dr, C0 + c + dc] = 1
        for a in range(5):             # arms: next piece colour, lines cleared
            g.arm(a, 7, h.hsv(HUE[nxt]), .25)
            g.arm(a, min(7, lines), h.hsv(HUE[nxt]), 1)

    while t < 300:
        rots = rotations(SHAPES[cur])
        best = max(((ri, c) for ri in range(len(rots)) for c in range(W)),
                   key=lambda rc: evaluate(rots[rc[0]], rc[1]))
        ri, c, r = 0, (W - 2) // 2, 0
        if not fits(rots[0], 0, c):
            break                        # topped out
        while (ri, c) != best:           # rotate first, then slide
            if ri != best[0]:
                ri = (ri + 1) % len(rots)
                g.press(1)
            elif c < best[1]:
                c += 1
                g.press(3)
            else:
                c -= 1
                g.press(0)
            for _ in range(2):
                draw((rots[ri], r, c))
                g.emit()
                t += 1
        g.press(2)
        while fits(rots[ri], r + 1, c):  # hard drop, one row per frame
            r += 1
            draw((rots[ri], r, c))
            g.emit()
            t += 1
        for dr, dc in rots[ri]:
            field[r + dr, c + dc] = 1
        full = [i for i in range(H) if field[i].all()]
        if full:
            for k in range(9):
                draw(blink=full if k % 3 < 2 else ())
                g.rainbow(k)
                g.emit()
                t += 1
            field = np.vstack([np.zeros((len(full), W), int), np.delete(field, full, 0)])
            lines += len(full)
        cur, nxt = nxt, next_piece()
    g.score_screen(lines)
    return g


# --------------------------------------------------------------- breakout ---
def breakout(L):
    g = Game(L, {0: "◀", 3: "▶"})
    bricks = {(r, c) for r in (1, 2, 3, 4, 5) for c in range(L.GW) if L.gmask[r, c]}
    PR = 14                              # paddle row (cols 4..12 on the board)
    pc = 8.0                             # paddle centre
    y, x, vy, vx = 12.5, 8.0, -.75, .2
    score, lives, t, miss_at = 0, 3, 0, 230
    flash = 0
    while t < 330 and lives:
        # paddle AI: follow the ball, until it lets one go
        target = x if t < miss_at or t > miss_at + 60 else 4
        if abs(target - pc) > .6:
            step = 1 if target > pc else -1
            if 4 + 1 <= pc + step <= 12 - 1:
                pc += step
                g.press(3 if step > 0 else 0)
        ny, nx = y + vy, x + vx
        r, c = int(round(ny)), int(round(nx))
        if (r, c) in bricks:
            bricks.discard((r, c))
            score += 1
            flash = 6
            vy = -vy
        else:
            if not inside(L, int(round(ny)), int(round(x))) and ny < PR:
                vy = -vy
            if not inside(L, int(round(y)), int(round(nx))) and ny < PR:
                vx = -vx
        if vy > 0 and y < PR - .5 <= y + vy and abs(x - pc) <= 1.6:
            vy = -abs(vy)
            vx = max(-.3, min(.3, vx + (x - pc) * .1))
        y, x = y + vy, x + vx
        if y > 15.4:                     # lost the ball
            lives -= 1
            for k in range(14):
                g.arms_all((1, 0, 0), 1 - k / 14)
                g.grid[PR, int(pc) - 1:int(pc) + 2] = 1 if k % 4 < 2 else 0
                g.emit()
                t += 1
            y, x, vy, vx = 12.5, pc, -.75, .2
            if not bricks:
                break
            continue
        g.walls()
        for b in bricks:
            g.grid[b] = .55
        for k in (-1, 0, 1):
            g.grid[PR, int(pc) + k] = .85
        g.grid[int(round(y)), int(round(x))] = 1
        for a in range(lives):
            g.arm(a, 7, (.1, .5, 1), .4)
        for a in range(5):
            g.arm(a, score // 5, (1, .7, .1), .9)
        if flash:
            g.edge[:] = flash / 8
            g.arms_all(h.hsv(score / 12), flash / 6)
            flash -= 1
        g.emit()
        t += 1
        if not bricks:
            break
    g.score_screen(score)
    return g


# ------------------------------------------------------------------- pong ---
def pong(L):
    g = Game(L, {0: "P1 ▲", 1: "P1 ▼", 2: "P2 ▲", 3: "P2 ▼"})
    rnd = random.Random(2)
    p1, p2 = 7.0, 7.0                    # paddle top row, 2 tall, rows 5..10
    y, x, vy, vx = 7.5, 8.0, .21, .45
    s1 = s2 = 0
    t, flash, who = 0, 0, None
    err1, err2 = 0, 0
    while max(s1, s2) < 3 and t < 500:
        for side in (0, 1):
            pad_y = p1 if side == 0 else p2
            coming = (vx < 0) if side == 0 else (vx > 0)
            err = err1 if side == 0 else err2
            target = (y + err if coming else 7.5) - .5
            if t % 3 == side and abs(target - pad_y) > .5:
                step = 1 if target > pad_y else -1
                if 5 <= pad_y + step <= 9:
                    if side == 0:
                        p1 += step
                    else:
                        p2 += step
                    g.press((1 if step > 0 else 0) + 2 * side)
        ny, nx = y + vy, x + vx
        if not inside(L, int(round(ny)), int(round(x))) and 0 < nx < L.GW - 1:
            vy = -vy
        for side, pad_y, col in ((0, p1, 0), (1, p2, L.GW - 1)):
            hit_col = col + (1 if side == 0 else -1)
            moving = vx < 0 if side == 0 else vx > 0
            if moving and (x - hit_col) * (nx - hit_col) <= 0 and pad_y - .6 <= y <= pad_y + 1.6:
                vx = -vx * 1.04
                vy += (y - pad_y - .5) * .1
                if side == 0:
                    err2 = rnd.choice([0, 0, 0, 2.5])
                else:
                    err1 = rnd.choice([0, 0, 0, 2.5])
        y, x = y + vy, x + vx
        if x < -.5 or x > L.GW - .5:      # point
            who = 1 if x > 0 else 0
            if who == 0:
                s1 += 1
            else:
                s2 += 1
            flash = 16
            y, x, vy, vx = 7.5, 8.0, rnd.choice([-.2, .2]), .45 if who == 1 else -.45
            err1 = err2 = 0
        g.walls()
        for k in (0, 1):
            g.grid[int(p1) + k, 0] = 1
            g.grid[int(p2) + k, L.GW - 1] = 1
        for r in range(L.GH):            # centre net
            if r % 2 == 0:
                g.grid[r, 8] = max(g.grid[r, 8], .12)
        g.grid[int(round(y)) % L.GH, int(round(min(max(x, 0), L.GW - 1)))] = 1
        g.arm(4, s1 * 2 + 1, (.1, .4, 1), .9)   # left arm: P1
        g.arm(3, s1 * 2 + 1, (.1, .4, 1), .9)
        g.arm(1, s2 * 2 + 1, (1, .1, .1), .9)   # right arm: P2
        g.arm(2, s2 * 2 + 1, (1, .1, .1), .9)
        g.arm(0, 7, (1, 1, 1), .15)
        if flash:
            col = (.1, .4, 1) if who == 0 else (1, .1, .1)
            for a in ((4, 3) if who == 0 else (1, 2)):
                g.arm(a, 7, col, flash / 16)
            g.edge[:] = flash / 30
            flash -= 1
        g.emit()
        t += 1
    for k in range(30):
        # final score "s1 - s2" on the grid
        for ch, c0 in ((str(s1), 2), (str(s2), 12)):
            rows = DIGITS[ch].split()
            for r in range(5):
                for c in range(3):
                    if rows[r][c] == "1":
                        g.grid[5 + r, c0 + c] = 1
        g.grid[7, 7:10] = .6
        winner = (4, 3) if s1 > s2 else (1, 2)
        col = (.1, .4, 1) if s1 > s2 else (1, .1, .1)
        for a in winner:
            g.arm(a, 7, col, .5 + .5 * (k % 6 < 3))
        g.emit()
    return g


# ----------------------------------------------------------------- flappy ---
def flappy(L):
    g = Game(L, {0: "flap"})
    rnd = random.Random(5)
    BC = 4                               # bird column
    y, vy = 7.0, 0.0
    pipes = []                           # [col (float), gap top row]
    score, t = 0, 0
    crash_at = 330
    while True:
        if t % 24 == 0:
            pipes.append([L.GW + 1.0, rnd.randint(3, 8)])
        for p in pipes:
            p[0] -= 1 / 3
        if pipes and pipes[0][0] < -1:
            pipes.pop(0)
        nextp = next((p for p in pipes if p[0] >= BC - .5), None)
        gap_mid = nextp[1] + 1.5 if nextp else 7.5
        if t < crash_at and vy >= 0 and y + vy * 3 > gap_mid + .4:
            vy = -.42
            g.press(0)
        vy = min(vy + .055, .6)
        y += vy
        for p in pipes:                  # passed a pipe: mark it, count it
            if len(p) == 2 and p[0] < BC - .5:
                p.append(True)
                score += 1
        r = int(round(y))
        hit = not inside(L, r, BC) or any(
            abs(p[0] - BC) < .6 and not (p[1] <= r <= p[1] + 3) for p in pipes)
        g.walls()
        for p in pipes:
            pc = int(round(p[0]))
            for rr in range(L.GH):
                if 0 <= pc < L.GW and not (p[1] <= rr <= p[1] + 3):
                    g.grid[rr, pc] = .4
        if 0 <= r < L.GH:
            g.grid[r, BC] = 1
            g.grid[min(L.GH - 1, max(0, r + (1 if vy < 0 else -1))), BC - 1] = .35
        for a in range(5):
            g.arm(a, min(7, score), h.hsv(.13), .8)
        g.edge[:] = .03
        g.emit()
        t += 1
        if hit and t > 20:
            break
    for k in range(18):
        if 0 <= r < L.GH and k % 6 < 3:
            g.grid[r, BC] = 1
        g.arms_all((1, .1, 0), 1 - k / 18)
        g.edge[:] = (1 - k / 18) * .5
        g.emit()
    g.score_screen(score)
    return g


# ------------------------------------------------------------- star chase ---
def star_chase(L):
    """Reaction game on the outline: stop the runner beside the green arm."""
    g = Game(L, {0: "stop"})
    n = len(L.edge)
    eth = L.mth[L.edge]
    tip_th = [L.rth[(L.arm == a) & (L.rank == 6)][0] for a in range(5)]

    def zone(a):
        d = np.abs((eth - tip_th[a] + np.pi) % (2 * np.pi) - np.pi)
        return set(np.nonzero(d < .09)[0])

    plan = [(1, True), (3, True), (0, False), (0, True), (4, True), (2, True)]
    pos, speed, level = 0.0, .9, 1
    for a, ok in plan:
        z = zone(a)
        centre = sorted(z)[len(z) // 2]
        # run at least most of a lap, then press on time (or a bit late)
        stop = pos + (centre - pos) % n + (n // 2 if (centre - pos) % n < n // 3 else 0)
        if not ok:
            stop += 4
        while pos < stop:
            pos = min(stop, pos + speed)
            p = int(pos) % n
            for k in range(6):
                g.edge[(p - k) % n] = max(g.edge[(p - k) % n], .55 ** k)
            for j in z:
                g.edge[j] = max(g.edge[j], .25)
            g.arm(a, 7, (.1, 1, .25), .55 + .45 * np.sin(pos / 2))
            g.digits(str(level), v=.7)
            g.emit()
        g.press(0)
        if ok:
            for k in range(16):            # hit: wave out from the arm, grid flash
                g.arm(a, 7, (.1, 1, .25), 1)
                g.rainbow(k, .9 * (1 - k / 16))
                g.grid[:] = (1 - k / 16) * .8
                g.digits(str(level + 1), v=1)
                g.edge[:] = (1 - k / 16) * .6
                g.emit()
            level += 1
            speed *= 1.2
        else:
            for k in range(16):            # miss: red
                g.arms_all((1, 0, 0), (k % 6 < 3) * .9)
                g.digits(str(level), v=.4)
                g.edge[int(pos) % n] = 1
                g.emit()
    g.score_screen(level)
    return g


GAMES = {
    "snake": ("SNAKE", "eat, grow, don't hit the edge", snake),
    "tetris": ("STARTRIS", "9-wide well in the octagon", tetris),
    "breakout": ("BREAKOUT", "bricks on top, paddle on the bottom row", breakout),
    "pong": ("PONG", "2 players, one on each side of the badge", pong),
    "flappy": ("FLAPPY", "one-button: flap through the gaps", flappy),
    "star_chase": ("STAR CHASE", "stop the runner beside the green arm", star_chase),
}


class PadRenderer(h.Renderer):
    """Board render plus the four touch pads in the caption."""

    def __init__(self, L):
        super().__init__(L)
        self.f_pad = ImageFont.truetype("/usr/share/fonts/truetype/DejaVuSans-Bold.ttf", 11)

    def frame(self, mono, rgb, pressed, title, sub, pads):
        out = self.render(mono, rgb, title, sub)
        dr = ImageDraw.Draw(out)
        w, gap = 50, 6
        x = h.S - 16 - 4 * w - 3 * gap
        for i in range(4):
            role = pads.get(i)
            on = i in pressed
            box = [x, h.S + 8, x + w, h.S + 30]
            if role is None:
                dr.rounded_rectangle(box, 8, outline=(45, 48, 56), width=1)
                dr.text(((box[0] + box[2]) / 2, h.S + 19), f"T{i + 1}", font=self.f_pad,
                        fill=(70, 74, 84), anchor="mm")
            else:
                dr.rounded_rectangle(box, 8, fill=(224, 70, 30) if on else None,
                                     outline=(224, 70, 30) if on else (95, 99, 110), width=2)
                dr.text(((box[0] + box[2]) / 2, h.S + 19), role, font=self.f_pad,
                        fill=(255, 255, 255) if on else (170, 174, 185), anchor="mm")
            x += w + gap
        return out


def main(args):
    opts = dict(a[2:].split("=", 1) if "=" in a else (a[2:], "") for a in args if a.startswith("--"))
    out = Path(opts.get("out", h.OUT / "games"))
    width = int(opts.get("width", h.S))
    names = [a for a in args if not a.startswith("--")] or list(GAMES)
    L = h.Layout()
    R = PadRenderer(L)
    out.mkdir(parents=True, exist_ok=True)
    for name in names:
        title, sub, fn = GAMES[name]
        g = fn(L)
        frames = [R.frame(m, c, p, title, sub, g.pads) for m, c, p in g.frames]
        if width != h.S:
            size = (width, round(width * (h.S + h.CAP) / h.S))
            frames = [f.resize(size, Image.LANCZOS) for f in frames]
        path = out / f"{name}.gif"
        h.save_gif(frames, path)
        msg = f"{path.name}: {len(frames)} frames, {path.stat().st_size // 1024} KiB"
        if "no-mp4" not in opts:
            h.save_mp4(frames, path.with_suffix(".mp4"))
            msg += f" (mp4 {path.with_suffix('.mp4').stat().st_size // 1024} KiB)"
        print(msg)


if __name__ == "__main__":
    main(sys.argv[1:])
