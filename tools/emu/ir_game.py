#!/usr/bin/env python3
"""Two emulated badges play four in a row over infrared, for every pairing
of the firmwares given (C with C, C with Rust, ...). The link loses
packets at random, and packets sent in the same frame collide and are both
lost. Each player presses ◀ / ▶ / B at random.

  python3 tools/emu/ir_game.py c/build/libhh27.so tools/fwsim/target/*/release/libfwsim.so

For every pairing and scenario, both badges must pair up (one starts, the
other does not), finish the same game (the UART log has the result and
the moves; one wins and the other loses, or both draw, with the same
moves). Scenarios: one badge invites while the other shows the demo; both
invite at once; and each again with a lossy link.
"""
import ctypes
import os
import random
import re
import shutil
import sys
import tempfile

FRAME = 16 * 17 + 71 + 35 * 3
PRESS, TAP = 0, 1
LEFT, B, RIGHT = 0, 1, 3


class Badge:
    def __init__(self, path, tmp, tag, seed):
        copy = os.path.join(tmp, f"{tag}-{os.path.basename(path)}")
        shutil.copy(path, copy)  # a second dlopen of the same file would share its state
        self.lib = ctypes.CDLL(copy)
        self.lib.emu_ir_tx.argtypes = [ctypes.POINTER(ctypes.c_uint8)]
        self.lib.emu_ir_rx.argtypes = [ctypes.c_char_p, ctypes.c_int]
        self.lib.emu_console.argtypes = [ctypes.c_char_p]
        self.lib.emu_usb.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.POINTER(ctypes.c_uint8)]
        self.buf = ctypes.create_string_buffer(FRAME)
        self.out = (ctypes.c_uint8 * 64)()
        self.lib.emu_init()
        self.lib.emu_seed(ctypes.c_uint32(seed))  # as each badge does with its chip ID

    def item(self, label):
        for i in range(64):
            n = self.lib.emu_usb(bytes([7, i]), 2, self.out)  # GET LABEL i
            if n > 3 and bytes(self.out[3:3 + self.out[2]]).decode() == label:
                return i
        sys.exit(f"no item {label}")

    def tap(self, button):
        self.lib.emu_event(PRESS, button)
        self.lib.emu_event(TAP, button)

    def frame(self):
        self.lib.emu_render(0, self.buf)
        n = self.lib.emu_ir_tx(self.out)
        return bytes(self.out[:n])


def play(pa, pb, both_invite, loss, seed, tmp, log_path, frames=30 * 300):
    rng = random.Random(seed)
    badges = [Badge(pa, tmp, "a", seed * 2 + 1), Badge(pb, tmp, "b", seed * 2 + 2)]
    for b in badges:
        b.lib.emu_console(f"show {b.item('4 IN A ROW')}".encode())
    sent = lost = 0
    linked = False
    for t in range(frames):
        if t == 10:
            badges[0].tap(LEFT)  # start a real game: it looks for a partner
            if both_invite:
                badges[1].tap(LEFT)
        if t % 15 == 0:  # the players wait until the badges have paired up, and stop after one game
            with open(log_path, errors="replace") as f:
                log = f.read()
            linked = len(re.findall("four: linked", log)) >= 2
            if len(re.findall(r"four: (?:you win|you lose|draw|link lost)", log)) >= 2:
                break
        if linked and t % 7 == 0:
            for b in badges:
                if rng.random() < 0.5:
                    b.tap(rng.choice([LEFT, RIGHT, B, B]))
        packets = [b.frame() for b in badges]
        if packets[0] and packets[1]:  # both at once: collide
            sent, lost = sent + 2, lost + 2
            continue
        for me, p in enumerate(packets):
            if not p:
                continue
            sent += 1
            if rng.random() < loss:
                lost += 1
            else:
                badges[1 - me].lib.emu_ir_rx(p, len(p))
    return sent, lost


def check(log):
    linked = re.findall(r"four: linked, (you|they) start", log)
    results = re.findall(r"four: (you win|you lose|draw), moves (\d+)", log)
    if "link lost" in log:
        return "link lost"
    if sorted(linked) != ["they", "you"]:
        return f"pairing: {linked}"
    if len(results) < 2:
        return f"no result on both badges: {results}"
    (r1, m1), (r2, m2) = results[:2]
    if m1 != m2:
        return f"the badges disagree on the moves: {m1} / {m2}"
    if sorted([r1, r2]) not in (["you lose", "you win"], ["draw", "draw"]):
        return f"results {r1} / {r2}"
    return None


def run(pa, pb, both_invite, loss, seed):
    """One game in a child's stderr: the firmwares log there."""
    with tempfile.TemporaryDirectory() as tmp:
        log_path = os.path.join(tmp, "log")
        saved = os.dup(2)
        fd = os.open(log_path, os.O_WRONLY | os.O_CREAT)
        os.dup2(fd, 2)
        try:
            sent, lost = play(pa, pb, both_invite, loss, seed, tmp, log_path)
        finally:
            os.dup2(saved, 2)
            os.close(fd)
        with open(log_path, errors="replace") as f:
            log = f.read()
    err = check(log)
    moves = re.findall(r"moves (\d+)", log)
    return err, sent, lost, moves[0] if moves else ""


def main(argv):
    if len(argv) == 7:  # one game, in a child process
        err, sent, lost, moves = run(argv[1], argv[2], argv[3] == "1", float(argv[4]), int(argv[5]))
        print(f"{err or 'ok'} ({sent} packets, {lost} lost, moves {moves})")
        return 1 if err else 0
    libs = argv[1:]
    if not libs:
        sys.exit(__doc__)
    import subprocess
    failed = total = 0
    for pa in libs:
        for pb in libs:
            for both in (0, 1):
                for loss in (0.0, 0.3):
                    for seed in (1, 2):
                        total += 1
                        r = subprocess.run([sys.executable, __file__, pa, pb, str(both), str(loss), str(seed), "x"],
                                           capture_output=True, text=True)
                        name = f"{os.path.basename(pa)} vs {os.path.basename(pb)}, " \
                               f"{'both invite' if both else 'one invites'}, loss {loss:.0%}, seed {seed}"
                        print(f"{name}: {r.stdout.strip() or r.stderr.strip()[-200:]}")
                        failed += r.returncode != 0
    print(f"ir_game: {total - failed} of {total} games played out")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
