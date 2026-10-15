#!/usr/bin/env python3
"""C / Rust parity: both firmwares, built as emulator libraries, get the
same inputs and must draw the same frames.

  python3 tools/emu/parity.py c/build/libhh27.so tools/fwsim/target/*/release/libfwsim.so

Only the screens without random numbers are compared bit for bit: the C
firmware has one shared random generator, Rust one per user, so anything
that draws random numbers (name tag sparkle, LIFE, FIRE, most games) differs
from the first frame without being wrong. What is compared here
(hotel, ripple, galaxy, heart, twinkle, breakout's demo, the victory
screen, the factory test) was identical at the time of writing; a change to
one firmware that is not made in the other shows up as the first frame that
differs. Each case runs in its own process, so no state carries over.
"""
import ctypes
import subprocess
import sys

FRAME = 16 * 17 + 71 + 35 * 3
ITEMS = ["HACKER HOTEL", "RIPPLE", "GALAXY", "HEART", "TWINKLE", "BREAKOUT", "VICTORY"]
FRAMES = 600


def load(path):
    lib = ctypes.CDLL(path)
    lib.emu_render.restype = ctypes.c_uint32
    lib.emu_render.argtypes = [ctypes.c_uint8, ctypes.c_char_p]
    lib.emu_console.argtypes = [ctypes.c_char_p]
    lib.emu_usb.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p]
    return lib


def label_index(lib, label):
    """the item's number, through the USB protocol's GET LABEL"""
    out = ctypes.create_string_buffer(64)
    for i in range(64):
        n = lib.emu_usb(bytes([7, i]), 2, out)
        if out.raw[0] == 0xFF:
            break
        if out.raw[3:3 + out.raw[2]].decode() == label:
            return i
    sys.exit(f"no item {label}")


def run_case(c_path, rust_path, case):
    libs = [load(c_path), load(rust_path)]
    for lib in libs:
        if case == "factory":
            lib.emu_fresh()
        lib.emu_init()
        if case != "factory":
            lib.emu_console(f"show {label_index(lib, case)}".encode())
    bufs = [ctypes.create_string_buffer(FRAME) for _ in libs]
    for t in range(FRAMES):
        for lib, buf in zip(libs, bufs):
            lib.emu_render(0, buf)
        if bufs[0].raw != bufs[1].raw:
            diff = [i for i in range(FRAME) if bufs[0].raw[i] != bufs[1].raw[i]]
            return f"{case}: frame {t} differs in {len(diff)} bytes, first at {diff[0]}"
    return None


def main(argv):
    if len(argv) == 4:  # one case, in a child process
        err = run_case(argv[1], argv[2], argv[3])
        if err:
            print(err)
        return 1 if err else 0
    if len(argv) != 3:
        sys.exit(__doc__)
    failed = 0
    for case in ITEMS + ["factory"]:
        r = subprocess.run([sys.executable, __file__, argv[1], argv[2], case],
                           stderr=subprocess.DEVNULL)
        failed += r.returncode != 0
    print(f"parity: {len(ITEMS) + 1 - failed} of {len(ITEMS) + 1} cases identical for {FRAMES} frames")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
