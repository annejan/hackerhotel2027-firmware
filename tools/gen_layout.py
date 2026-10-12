#!/usr/bin/env python3
"""Generate src/generated.rs (LED matrix map + lookup tables) from the KiCad PCB.

Needs the pcbnew Python module (ships with KiCad):

    python3 tools/gen_layout.py [path/to/hackerhotel2027.kicad_pcb]

Without an argument it looks for $HH27_HW, then for a hackerhotel2027-hardware
checkout next to this repo.

How the matrix is wired (traced from the PCB, not assumed):
  * every LED anode goes through a series resistor to a CH643 PAx pin  -> PWM channel x
  * every LED cathode goes straight to a CH643 PBx pin                  -> COM line x
  * the RGB LEDs are common-anode: one PWM channel, three COMs (R, G, B)
A "slot" is com * PWM_CHANNELS + pwm, i.e. the frame buffer is COM-major.
"""
import math
import os
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import pcbnew

HERE = Path(__file__).resolve().parent
REPO = HERE.parent


def find_pcb():
    if len(sys.argv) > 1:
        return Path(sys.argv[1])
    name = "hackerhotel2027-hardware"
    cands = [Path(os.environ["HH27_HW"])] if "HH27_HW" in os.environ else []
    cands += [REPO.parent / name, REPO.parent.parent / name]
    for d in cands:
        if (d / "hackerhotel2027.kicad_pcb").exists():
            return d / "hackerhotel2027.kicad_pcb"
    sys.exit(f"can't find {name}; clone it next to this repo, set HH27_HW or pass the .kicad_pcb")


PCB = find_pcb()
OUT = HERE.parent / "src" / "generated.rs"

RGB_PAD_COLOUR = {"1": "r", "4": "g", "3": "b"}   # from library/led.kicad_sym (RED=1 GREEN=4 BLUE=3, A=2)
GRID_PITCH = 2.0                                   # mm

b = pcbnew.LoadBoard(str(PCB))
mm = pcbnew.ToMM
fps = {fp.GetReference(): fp for fp in b.GetFootprints()}

# MCU net -> port pin, from the CH643 pad pin functions ("PA15/TX2_" ...)
mcu = fps["U1"]
pin_of_net = {}
for pad in mcu.Pads():
    m = re.match(r"P([ABC])(\d+)", pad.GetPinFunction())
    if m and pad.GetNetname():
        pin_of_net[pad.GetNetname()] = (m.group(1), int(m.group(2)))

# follow series resistors: LED-side net -> MCU net
via_res = {}
for ref, fp in fps.items():
    if ref.startswith("R"):
        nets = [p.GetNetname() for p in fp.Pads()]
        if len(nets) == 2:
            a, c = nets
            if a in pin_of_net:
                via_res[c] = a
            elif c in pin_of_net:
                via_res[a] = c


def port_pin(net):
    net = via_res.get(net, net)
    return pin_of_net.get(net)


def pwm_of(net):
    pp = port_pin(net)
    assert pp and pp[0] == "A", f"anode net {net} does not reach a PA pin ({pp})"
    return pp[1]


def com_of(net):
    pp = port_pin(net)
    assert pp and pp[0] == "B", f"cathode net {net} does not reach a PB pin ({pp})"
    return pp[1]


mono, rgb = [], []
for ref, fp in fps.items():
    lib = str(fp.GetFPID().GetLibItemName())
    if not ref.startswith("D"):
        continue
    pads = {p.GetNumber(): p.GetNetname() for p in fp.Pads()}
    pos = fp.GetPosition()
    x, y = mm(pos.x), mm(pos.y)
    if "B1515" in lib:
        rgb.append(dict(ref=ref, x=x, y=y, pwm=pwm_of(pads["2"]),
                        **{c: com_of(pads[n]) for n, c in RGB_PAD_COLOUR.items()}))
    elif "0402" in lib:
        mono.append(dict(ref=ref, x=x, y=y, pwm=pwm_of(pads["2"]), com=com_of(pads["1"])))

PWM = max(max(l["pwm"] for l in mono + rgb) + 1, 1)
COM = max(max(l["com"] for l in mono), max(max(l["r"], l["g"], l["b"]) for l in rgb)) + 1


def slot(com, pwm):
    return com * PWM + pwm


used = Counter([slot(l["com"], l["pwm"]) for l in mono] +
               [slot(l[c], l["pwm"]) for l in rgb for c in "rgb"])
dup = [s for s, n in used.items() if n > 1]
assert not dup, f"slots driven by more than one LED: {dup}"

cx = sum(l["x"] for l in rgb) / len(rgb)
cy = sum(l["y"] for l in rgb) / len(rgb)

# split mono into centre grid and outline
def phase(vals):
    return Counter(round(v % GRID_PITCH, 1) for v in vals).most_common(1)[0][0]


px, py = phase([l["x"] for l in mono]), phase([l["y"] for l in mono])
grid, edge = {}, []
for l in mono:
    c, r = (l["x"] - px) / GRID_PITCH, (l["y"] - py) / GRID_PITCH
    if abs(l["x"]) < 18 and abs(l["y"]) < 18 and abs(c - round(c)) < .1 and abs(r - round(r)) < .1:
        grid[(round(r), round(c))] = l
    else:
        edge.append(l)
r0 = min(r for r, c in grid)
c0 = min(c for r, c in grid)
grid = {(r - r0, c - c0): l for (r, c), l in grid.items()}
GW = max(c for r, c in grid) + 1
GH = max(r for r, c in grid) + 1


def polar(l):
    dx, dy = l["x"] - cx, l["y"] - cy
    rad = math.hypot(dx, dy)
    ang = math.atan2(dx, -dy) % (2 * math.pi)        # 0 = up, clockwise
    return min(255, round(rad * 4)), round(ang / (2 * math.pi) * 256) % 256


edge.sort(key=lambda l: polar(l)[1])

# RGB: arm by angle (top arm = 0, clockwise), rank by distance from centre
for l in rgb:
    l["arm"] = ((polar(l)[1] + 26) % 256) * 5 // 256
arms = defaultdict(list)
for l in rgb:
    arms[l["arm"]].append(l)
for a, ls in arms.items():
    for k, l in enumerate(sorted(ls, key=lambda l: polar(l)[0])):
        l["rank"] = k
rgb.sort(key=lambda l: (l["arm"], l["rank"]))
assert sorted(len(v) for v in arms.values()) == [7] * 5, {a: len(v) for a, v in arms.items()}

NONE = 0xFFFF
pol = [(0, 0)] * (PWM * COM)
for l in mono:
    pol[slot(l["com"], l["pwm"])] = polar(l)
for l in rgb:
    for c in "rgb":
        pol[slot(l[c], l["pwm"])] = polar(l)

gamma = [round(255 * (i / 255) ** 2.2) for i in range(256)]
sin8 = [round(127.5 + 127.5 * math.sin(2 * math.pi * i / 256)) for i in range(256)]


def rows(items, per, fmt="{}"):
    items = [fmt.format(i) for i in items]
    return "\n".join("    " + ", ".join(items[i:i + per]) + "," for i in range(0, len(items), per))


o = []
o.append(f"// @generated by tools/gen_layout.py from {PCB.name}. Do not edit by hand.\n")
o.append("#![allow(dead_code)]\n")
o.append(f"/// PWM channels: PA0..PA{PWM - 1}, LED anodes (through series resistors).")
o.append(f"pub const PWM_CHANNELS: usize = {PWM};")
o.append(f"/// COM lines: PB0..PB{COM - 1}, LED cathodes, scanned one at a time.")
o.append(f"pub const COMS: usize = {COM};")
o.append("pub const SLOTS: usize = PWM_CHANNELS * COMS;")
o.append("/// Frame buffer index (`com * PWM_CHANNELS + pwm`); NONE = no LED there.")
o.append("pub type Slot = u16;")
o.append(f"pub const NONE: Slot = 0x{NONE:04X};\n")
o.append(f"pub const GRID_W: usize = {GW};")
o.append(f"pub const GRID_H: usize = {GH};")
o.append(f"/// Centre grid, {len(grid)} mono LEDs, 2 mm pitch, octagonal (corners are NONE).")
o.append("pub const GRID: [[Slot; GRID_W]; GRID_H] = [")
for r in range(GH):
    vals = [slot(grid[(r, c)]["com"], grid[(r, c)]["pwm"]) if (r, c) in grid else NONE
            for c in range(GW)]
    o.append("    [" + ", ".join(f"{v:>6}" if v != NONE else "  NONE" for v in vals) + "],")
o.append("];\n")
o.append(f"/// Outline LEDs ({len(edge)}), clockwise starting at the top.")
o.append(f"pub const EDGE: [Slot; {len(edge)}] = [")
o.append(rows([slot(l["com"], l["pwm"]) for l in edge], 12))
o.append("];\n")
o.append("#[derive(Clone, Copy)]")
o.append("pub struct RgbLed {")
o.append("    pub r: Slot,")
o.append("    pub g: Slot,")
o.append("    pub b: Slot,")
o.append("    /// 0 = top arm, then clockwise")
o.append("    pub arm: u8,")
o.append("    /// 0 = nearest the centre .. 6 = tip")
o.append("    pub rank: u8,")
o.append("}\n")
o.append(f"pub const RGB: [RgbLed; {len(rgb)}] = [")
for l in rgb:
    o.append(f"    RgbLed {{ r: {slot(l['r'], l['pwm']):>3}, g: {slot(l['g'], l['pwm']):>3}, "
             f"b: {slot(l['b'], l['pwm']):>3}, arm: {l['arm']}, rank: {l['rank']} }}, // {l['ref']}")
o.append("];\n")
o.append("/// Polar position of every slot, seen from the board centre:")
o.append("/// (radius in 0.25 mm, angle in 1/256 turn, 0 = up, clockwise).")
o.append("pub const POLAR: [(u8, u8); SLOTS] = [")
o.append(rows([f"({r}, {a})" for r, a in pol], 8))
o.append("];\n")
o.append("/// 2.2 gamma, applied once when a frame is handed to the display.")
o.append("pub const GAMMA: [u8; 256] = [")
o.append(rows(gamma, 16))
o.append("];\n")
o.append("/// sin, 0..=255 in, 0..=255 out (128 = zero crossing).")
o.append("pub const SIN8: [u8; 256] = [")
o.append(rows(sin8, 16))
o.append("];")

OUT.write_text("\n".join(o) + "\n")
print(f"{OUT}: {PWM} PWM x {COM} COM, {len(grid)} grid ({GW}x{GH}), {len(edge)} edge, "
      f"{len(rgb)} RGB, {sum(used.values())}/{PWM * COM} slots used")
