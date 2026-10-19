#!/usr/bin/env bash
# docs_gifs.sh
# Records the firmware on the PC (tools/fwsim; the name swap from two C
# emulator libraries) and writes every docs/fw/*.gif with fwgif.py. Run it
# from the repo root after tools/sim/hh2027.py --refresh, or after a
# firmware change that alters what one of the GIFs shows.
set -euo pipefail

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

B="--buttons=◀,B,A,▶"

# rec NAME FRAMES EVENT...: fwsim record into $tmp/NAME.bin
rec() {
    (cd tools/fwsim && cargo run -q --release -- record "$tmp/$1.bin" "${@:2}") 2>"$tmp/$1.log" ||
        { cat "$tmp/$1.log" >&2; exit 1; }
}
# gif NAME TITLE SUBTITLE [OPTION...]: $tmp/NAME.bin to docs/fw/NAME.gif
gif() {
    python3 tools/sim/fwgif.py "$tmp/$1.bin" "docs/fw/$1.gif" "${@:2}" --width=260 | tail -1
}
# menu N: the events that open the menu (hold A), go N items on (▶) and pick (A)
menu() {
    local ev="1:hold2" k
    for ((k = 0; k < $1; k++)); do ev+=" $((2 + k)):tap3"; done
    echo "$ev $((3 + $1)):tap2"
}

# the name tag with a name and challenge progress (levels per arm, 3 bits each)
(cd tools/fwsim && BADGE_NAME="ANNE JAN" cargo build -q --release)
P=$((3 | 5 << 3 | 2 << 6 | 4 << 9 | 1 << 12))
BADGE_NAME="ANNE JAN" rec progress 240 0:progress$P
gif progress "NAME TAG" "arms: the five challenges, 5 levels each" $B --step=2

(cd tools/fwsim && cargo build -q --release)

rec hotel 215 $(menu 1)
rec startris 255 $(menu 9)
rec breakout 255 $(menu 10)
rec chase 255 $(menu 13)
gif hotel "HACKER HOTEL 2027" "" --buttons=,,light, --skip=15
gif startris "STARTRIS" "" --buttons=◀,drop,rot,▶ --skip=15
gif breakout "BREAKOUT" "" --buttons=◀,,,▶ --skip=15
gif chase "STAR CHASE" "" --buttons=stop,stop,,stop --skip=15

rec selftest 1140 15:down1 15:down2 105:selftest 106:up1 106:up2 225:tap3 345:tap3 \
    465:tap3 540:tap3 630:tap3 670:down1 700:up1 705:down0 735:up0 740:down3 760:up3 \
    765:tap3 855:tap3 1005:tap3 1040:charge 1110:hold2
gif selftest "BRING-UP TEST" "hold A+B 3 s, ◀ ▶ pick a test" $B --step=2

rec factory 730 0:factory 165:tap1 360:tap1 405:press0 420:press1 435:press2 450:press3 540:charge
gif factory "FACTORY TEST" "fresh badge: LED, ROW, KEY, USB, IR" $B --step=2

rec sponsors 640 0:sponsors
gif sponsors "SPONSORS" "plays once, on the boot after the factory test" $B --step=3

rec whack 420 0:show14
rec simon 600 0:show15
rec maze 450 0:show16
rec hilo 360 0:show17
gif whack "WHACK-A-MOLE" "stop it before the centre" $B --step=2 --skip=30
gif simon "SIMON" "repeat the legs" $B --step=2
gif maze "MAZE" "the arm points to the exit" $B --step=2
gif hilo "HIGHER LOWER" "▶ higher, ◀ lower" $B --step=2

rec four 470 0:show18 20:tap0 70:tap1 110:tap3 125:tap1 165:tap0 175:tap0 190:tap1 230:tap3 \
    240:tap3 250:tap1 290:tap0 300:tap1 340:tap3 350:tap1 390:tap1 430:tap1
gif four "4 IN A ROW" "against the badge, or another badge over IR" $B --step=3 --skip=12

ALL=$((5 | 5 << 3 | 5 << 6 | 5 << 9 | 5 << 12))
rec victory 330 0:progress$ALL 1:show19
gif victory "VICTORY" "25 / 25" $B --step=2

# name swap: two C emulator libraries, each its own badge, passing IR packets;
# B on the first one's name tag at frame 16
make -s -C c emu
cp c/build/libhh27.so "$tmp/a.so"
cp c/build/libhh27.so "$tmp/b.so"
python3 - "$tmp" <<'EOF'
import ctypes, sys
d = sys.argv[1]
def load(path, name):
    lib = ctypes.CDLL(path)
    lib.emu_render.argtypes = [ctypes.c_uint8, ctypes.POINTER(ctypes.c_uint8)]
    lib.emu_ir_tx.argtypes = [ctypes.POINTER(ctypes.c_uint8)]
    lib.emu_ir_rx.argtypes = [ctypes.c_char_p, ctypes.c_int]
    lib.emu_console.argtypes = [ctypes.c_char_p]
    lib.emu_init()
    lib.emu_console(name.encode())
    return lib
a, b = load(d + "/a.so", "name ANNE JAN"), load(d + "/b.so", "name BADGE")
buf = (ctypes.c_uint8 * (16 * 17 + a.emu_edge_leds() + 35 * 3))()
def tx(lib):
    o = (ctypes.c_uint8 * 64)()
    n = lib.emu_ir_tx(o)
    return bytes(o[:n])
rec = bytearray()
for f in range(210):
    if f == 16:
        a.emu_event(0, 1)  # press + tap B
        a.emu_event(1, 1)
    pressed = 1 << 1 if 16 <= f < 24 else 0  # lit in the caption
    for me, other in ((a, b), (b, a)):
        me.emu_render(0, buf)
        if me is a:
            rec += bytes(buf) + bytes([pressed << 4, 32])
        p = tx(me)
        if p:
            other.emu_ir_rx(p, len(p))
open(d + "/swap.bin", "wb").write(rec)
EOF
gif swap "NAME SWAP" "" --buttons=,send,, --step=2
