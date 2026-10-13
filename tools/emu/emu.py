#!/usr/bin/env python3
"""Badge emulator: runs a firmware built as a shared library on the PC.

  make -C c emu                      # C firmware   -> c/build/libhh27.so
  (cd tools/fwsim && cargo build --release)   # Rust -> tools/fwsim/target/*/release/libfwsim.so
  python3 tools/emu/emu.py [c|rust|path/to/lib.so] [--web] [--fresh]

--fresh starts as a freshly flashed badge: the factory test first, and
after a reset (R) the sponsor reel, as on a real badge.

--web also serves the badge's setup page (docs/webusb) on
http://localhost:8027/ and passes its packets to the firmware, as WebUSB
would on a real badge.

Infrared: every emulator on this PC shares one "room" (UDP
broadcast on 127.255.255.255:8028), so start two and press B on one's
name tag: they swap names. Lines typed on stdin go to the badge's UART
console (try `help`); its log appears on stderr. --no-ir leaves the room.

The library holds the hardware-independent firmware (app, menu, games,
animations; interface in badge_emu.h). This program stands in for the rest:
the buttons (with the same press / tap / hold / chord rules as buttons.rs),
the charger, and the LEDs, drawn on a KiCad render of the board.

Keys: Left / Right = ◀ / ▶ (the front buttons), Up or A = A, Down or B = B
(the buttons on the back), or click the buttons on the board.
C = USB power on/off, R = reset, Esc = quit.
"""
import ctypes
import json
import os
import socket
import queue
import sys
import threading
import time
import webbrowser
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import tkinter as tk
from pathlib import Path

import numpy as np
from PIL import Image, ImageTk

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
sys.path.insert(0, str(REPO / "tools" / "sim"))
import hh2027 as h  # noqa: E402

GH, GW, NE, NR = 16, 17, 71, 35
FRAME_BYTES = GH * GW + NE + NR * 3
HOLD_MS, BOOT_MS = 600, 3000          # as buttons.rs
PRESS, TAP, HOLD, CHARGING, BOOTLOADER, SELFTEST = range(6)

# button index: 0 ◀, 1 B, 2 A, 3 ▶ (see src/buttons.rs)
KEYS = {"Left": 0, "Down": 1, "b": 1, "B": 1, "Up": 2, "a": 2, "A": 2, "Right": 3}
BUTTON_NAMES = ["◀", "B", "A", "▶"]
BUTTON_REFS = ["SW3", "SW1", "SW2", "SW4"]


def find_lib(which):
    if which and Path(which).suffix == ".so":
        return Path(which)
    cands = {
        "c": [REPO / "c/build/libhh27.so"],
        "rust": sorted(REPO.glob("tools/fwsim/target/*/release/libfwsim.so"))
        + sorted(REPO.glob("tools/fwsim/target/release/libfwsim.so")),
    }
    for name in ([which] if which else ["c", "rust"]):
        for p in cands.get(name, []):
            if p.exists():
                return p
    sys.exit("no firmware library found: run `make -C c emu` or "
             "`cd tools/fwsim && cargo build --release`, or pass a .so path")


class Firmware:
    def __init__(self, path, fresh=False):
        self.lib = lib = ctypes.CDLL(str(path))
        lib.emu_abi.restype = ctypes.c_uint32
        lib.emu_firmware.restype = ctypes.c_char_p
        lib.emu_fps.restype = ctypes.c_uint32
        lib.emu_event.argtypes = [ctypes.c_uint8, ctypes.c_uint8]
        lib.emu_render.argtypes = [ctypes.c_uint8, ctypes.POINTER(ctypes.c_uint8)]
        lib.emu_render.restype = ctypes.c_uint32
        lib.emu_brightness.restype = ctypes.c_uint8
        if lib.emu_abi() != 1:
            sys.exit(f"{path}: emulator interface {lib.emu_abi()}, expected 1")
        self.name = lib.emu_firmware().decode()
        self.fps = lib.emu_fps()
        self.buf = (ctypes.c_uint8 * FRAME_BYTES)()
        self.has_usb = hasattr(lib, "emu_usb")
        if self.has_usb:
            lib.emu_usb.argtypes = [ctypes.POINTER(ctypes.c_uint8), ctypes.c_int, ctypes.POINTER(ctypes.c_uint8)]
            lib.emu_usb.restype = ctypes.c_int
        self.has_ir = hasattr(lib, "emu_ir_tx") and hasattr(lib, "emu_ir_rx")
        if self.has_ir:
            lib.emu_ir_tx.argtypes = [ctypes.POINTER(ctypes.c_uint8)]
            lib.emu_ir_tx.restype = ctypes.c_int
            lib.emu_ir_rx.argtypes = [ctypes.c_char_p, ctypes.c_int]
        self.has_console = hasattr(lib, "emu_console")
        if self.has_console:
            lib.emu_console.argtypes = [ctypes.c_char_p]
        if fresh and hasattr(lib, "emu_fresh"):
            lib.emu_fresh()
        lib.emu_init()

    def usb(self, packet):
        """One packet of the badge's USB protocol; returns the reply."""
        data = (ctypes.c_uint8 * 64).from_buffer_copy(packet[:64].ljust(64, b"\0"))
        out = (ctypes.c_uint8 * 64)()
        n = self.lib.emu_usb(data, min(len(packet), 64), out)
        return bytes(out[:n])

    def ir_tx(self):
        """The packet the firmware wants to send over IR, or b""."""
        out = (ctypes.c_uint8 * 64)()
        return bytes(out[:self.lib.emu_ir_tx(out)])

    def ir_rx(self, packet):
        self.lib.emu_ir_rx(packet, len(packet))

    def console(self, line):
        self.lib.emu_console(line.encode(errors="replace"))

    def event(self, kind, arg=0):
        self.lib.emu_event(kind, arg)

    def render(self, held):
        rebooted = self.lib.emu_render(held, self.buf)
        return rebooted, np.frombuffer(self.buf, np.uint8).copy()


WEB_PORT = 8027
WEB_ROOT = REPO / "docs" / "webusb"


class WebBridge:
    """The setup page and a /usb endpoint on localhost. Packets are handed
    to the Tk thread through a queue, so the firmware only ever runs there."""

    def __init__(self):
        self.requests = queue.Queue()
        bridge = self

        class Handler(SimpleHTTPRequestHandler):
            def __init__(self, *a, **kw):
                super().__init__(*a, directory=str(WEB_ROOT), **kw)

            def do_POST(self):
                if self.path.rstrip("/") != "/usb":
                    return self.send_error(404)
                packet = self.rfile.read(int(self.headers.get("Content-Length", 0)))
                box = {"done": threading.Event()}
                bridge.requests.put((packet, box))
                if not box["done"].wait(2):
                    return self.send_error(504, "emulator busy")
                self.send_response(200)
                self.send_header("Content-Type", "application/octet-stream")
                self.send_header("Content-Length", str(len(box["reply"])))
                self.end_headers()
                self.wfile.write(box["reply"])

            def log_message(self, *a):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", WEB_PORT), Handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def serve(self, fw):
        """Answer the waiting packets (from the Tk thread)."""
        while True:
            try:
                packet, box = self.requests.get_nowait()
            except queue.Empty:
                return
            box["reply"] = fw.usb(packet)
            box["done"].set()


IR_ADDR = ("127.255.255.255", 8028)


class IrRoom:
    """The emulators' shared infrared: a UDP broadcast that only this PC
    sees. Each packet carries a random sender id, so an emulator skips
    its own."""

    def __init__(self):
        self.me = os.urandom(4)
        self.sock = s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        s.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
        s.bind(IR_ADDR)
        s.setblocking(False)

    def send(self, packet):
        self.sock.sendto(self.me + packet, IR_ADDR)

    def received(self):
        while True:
            try:
                data = self.sock.recv(256)
            except BlockingIOError:
                return
            if data[:4] != self.me:
                yield data[4:]


class StdinConsole:
    """Lines from stdin, for the badge's UART console, read on a thread
    and handed to the Tk thread."""

    def __init__(self):
        self.lines = queue.Queue()
        threading.Thread(target=self.read, daemon=True).start()

    def read(self):
        for line in sys.stdin:
            self.lines.put(line.rstrip("\r\n"))


class Buttons:
    """The button logic of buttons.rs: press on press-down, tap on a short
    release, hold after 600 ms, buttons down together form a chord (no tap or
    hold of their own), ◀ + ▶ for 3 s = boot ROM, A + B for 3 s = the
    bring-up test."""

    def __init__(self, send):
        self.send = send
        self.down = [None] * 4          # time the button went down
        self.chord = [False] * 4
        self.held_sent = [False] * 4
        self.chord_sent = {}

    def press(self, btn):
        if self.down[btn] is None:
            self.down[btn] = time.monotonic()
            self.held_sent[btn] = False
            self.send(PRESS, btn)

    def release(self, btn):
        t0 = self.down[btn]
        if t0 is None:
            return
        self.down[btn] = None
        if not self.chord[btn] and (time.monotonic() - t0) * 1000 < HOLD_MS:
            self.send(TAP, btn)
        self.chord[btn] = False

    def poll(self):
        now = time.monotonic()
        downs = [p for p in range(4) if self.down[p] is not None]
        if len(downs) > 1:
            for p in downs:
                self.chord[p] = True
        for p in downs:
            if not self.chord[p] and not self.held_sent[p] and (now - self.down[p]) * 1000 >= HOLD_MS:
                self.held_sent[p] = True
                self.send(HOLD, p)
        for (i, j), event in (((0, 3), BOOTLOADER), ((2, 1), SELFTEST)):
            if self.down[i] is not None and self.down[j] is not None:
                if not self.chord_sent.get(event) and (now - max(self.down[i], self.down[j])) * 1000 >= BOOT_MS:
                    self.chord_sent[event] = True
                    self.send(event, 0)
            else:
                self.chord_sent[event] = False

    def held(self):
        return sum(1 << p for p in range(4) if self.down[p] is not None)


class FastRenderer:
    """The look of hh2027.Renderer, fast enough for live use: each LED's glow
    is a precomputed sprite added where the LED is lit."""

    def __init__(self):
        self.L = L = h.Layout()
        self.buttons = json.loads(h.LED_JSON.read_text()).get("buttons", [])
        base = h.Renderer(L)            # board render, dimmed, and its albedo
        self.base = base.base.astype(np.float32)
        albedo = base.albedo[:, :, 0]
        S = self.base.shape[0]

        def sprites(px, core, hot, halo, gains):
            out = []
            for x, y in px:
                r = int(3 * halo)
                xi, yi = int(round(x)), int(round(y))
                x0, x1, y0, y1 = max(0, xi - r), min(S, xi + r + 1), max(0, yi - r), min(S, yi + r + 1)
                yy, xx = np.mgrid[y0:y1, x0:x1].astype(np.float32)
                d2 = (xx - x) ** 2 + (yy - y) ** 2

                def g(s):  # peak 1 for a fully lit LED, as hh2027.Renderer.glow
                    return np.exp(-d2 / (2 * s * s))

                col = gains[0] * g(core) + gains[2] * g(halo) * (.3 + .9 * albedo[y0:y1, x0:x1])
                out.append((y0, y1, x0, x1, col[..., None], (gains[1] * g(hot))[..., None]))
            return out

        # same widths and gains as hh2027.Renderer (core, hot, halo)
        self.mono = sprites(L.mono_px, 1.3, .8, 5.5, (1.3, .5, .22))
        self.rgb = sprites(L.rgb_px, 2.6, 1.4, 13, (1.8, .8, .42))
        self.rgb_index = [int(np.nonzero((L.arm == k // 7) & (L.rank == k % 7))[0][0]) for k in range(NR)]

    def render(self, frame, gain=1.0):
        L = self.L
        mono = L.grid_to_mono(frame[:GH * GW].reshape(GH, GW) / 255)
        for j, i in enumerate(L.edge):
            mono[i] = frame[GH * GW + j] / 255
        rgb = np.zeros((NR, 3))
        rgb[self.rgb_index] = frame[GH * GW + NE:].reshape(NR, 3) / 255
        img = self.base.copy()
        for v, (y0, y1, x0, x1, col, hot) in zip(mono * gain, self.mono):
            if v > .004:
                img[y0:y1, x0:x1] += col * (h.MONO_COL * v) + hot * v
        for c, (y0, y1, x0, x1, col, hot) in zip(rgb * gain, self.rgb):
            if c.max() > .004:
                img[y0:y1, x0:x1] += col * c + hot * c.max()
        return Image.fromarray((np.clip(img, 0, 1) * 255).astype(np.uint8))


class App:
    def __init__(self, fw, web=None, ir=None, console=None):
        self.fw = fw
        self.web = web
        self.ir = ir
        self.console = console
        self.buttons = Buttons(fw.event)
        self.renderer = FastRenderer()
        self.charging = False
        self.message, self.message_until = "", 0
        self.release_pending = {}

        self.root = root = tk.Tk()
        root.title(f"HH2027 starfish badge — {fw.name} firmware")
        root.configure(bg="#111318")
        S = self.renderer.base.shape[0]
        self.canvas = tk.Canvas(root, width=S, height=S + 72, bg="#111318", highlightthickness=0)
        self.canvas.pack()
        self.image_id = self.canvas.create_image(0, 0, anchor="nw")
        # the buttons where they are on the board; the two on the back
        # (A, B) are drawn dashed, at the tips of the side arms
        self.rings = []
        L, where = self.renderer.L, {b["ref"]: b for b in self.renderer.buttons}
        for i, ref in enumerate(BUTTON_REFS):
            b = where.get(ref)
            x, y = L.to_px(b["x"], b["y"]) if b else (40 + i * 40, S - 30)
            back = bool(b and b["back"])
            r = self.canvas.create_oval(x - 15, y - 15, x + 15, y + 15, outline="#5f6370", width=3,
                                        dash=(4, 3) if back else None)
            # labels above the back ones, beside the front ones (outwards)
            tx, ty, anchor = (x, y - 20, "s") if back else (x - 20, y, "e") if x < S / 2 else (x + 20, y, "w")
            t = self.canvas.create_text(tx, ty, anchor=anchor, fill="#c8ccd4",
                                        text=BUTTON_NAMES[i] + (" (back)" if back else ""),
                                        font=("DejaVu Sans", 11, "bold"))
            for item in (r, t):
                self.canvas.tag_bind(item, "<ButtonPress-1>", lambda e, p=i: self.buttons.press(p))
                self.canvas.tag_bind(item, "<ButtonRelease-1>", lambda e, p=i: self.buttons.release(p))
            self.rings.append(r)
        self.status = self.canvas.create_text(16, S + 20, anchor="w", fill="#c8ccd4",
                                              font=("DejaVu Sans", 9))
        self.help = self.canvas.create_text(16, S + 40, anchor="w", fill="#7d838f", font=("DejaVu Sans", 9),
                                            text="keys: ← ◀  → ▶  ↑ or A: A  ↓ or B: B  ·  hold A: menu")
        self.help2 = self.canvas.create_text(16, S + 58, anchor="w", fill="#7d838f", font=("DejaVu Sans", 9),
                                             text="hold ◀+▶ 3 s: boot ROM  ·  C: USB power  ·  R: reset  ·  Esc: quit")
        root.bind("<KeyPress>", self.key_down)
        root.bind("<KeyRelease>", self.key_up)
        self.period = 1 / fw.fps
        self.next = time.monotonic()
        self.fps_t, self.fps_n, self.fps = time.monotonic(), 0, 0.0
        self.tick()

    # Tk repeats keys while held (press/release pairs): a release only
    # counts if no press follows within a few milliseconds.
    def key_down(self, e):
        if e.keysym in KEYS:
            btn = KEYS[e.keysym]
            pending = self.release_pending.pop(btn, None)
            if pending:
                self.root.after_cancel(pending)
            self.buttons.press(btn)
        elif e.keysym.lower() == "c":
            self.charging = not self.charging
            self.fw.event(CHARGING, int(self.charging))
            self.say("USB power " + ("on: charging" if self.charging else "off"))
        elif e.keysym.lower() == "r":
            self.fw.lib.emu_init()
            self.say("reset")
        elif e.keysym == "Escape":
            self.root.destroy()

    def key_up(self, e):
        if e.keysym in KEYS:
            btn = KEYS[e.keysym]
            self.release_pending[btn] = self.root.after(40, lambda: self.key_released(btn))

    def key_released(self, btn):
        self.release_pending.pop(btn, None)
        self.buttons.release(btn)

    def say(self, text, seconds=2.5):
        self.message, self.message_until = text, time.monotonic() + seconds

    def tick(self):
        if self.web:
            self.web.serve(self.fw)
        if self.console:
            while not self.console.lines.empty():
                self.fw.console(self.console.lines.get())
        if self.ir:
            for packet in self.ir.received():
                self.fw.ir_rx(packet)
                self.say("IR: received")
            packet = self.fw.ir_tx()
            if packet:
                self.ir.send(packet)
                self.say("IR: sent")
        self.buttons.poll()
        held = self.buttons.held()
        rebooted, frame = self.fw.render(held)
        if rebooted:
            self.say("USB boot ROM: ready for `wchisp flash` (powering on again)", 3)
            self.fw.lib.emu_init()
            rebooted, frame = self.fw.render(0)
        b = self.fw.lib.emu_brightness()
        # show brightness the way it feels: night mode dim but visible
        gain = max(.18, (b / 255) ** .45)
        self.photo = ImageTk.PhotoImage(self.renderer.render(frame, gain))
        self.canvas.itemconfigure(self.image_id, image=self.photo)
        for i, r in enumerate(self.rings):
            on = held >> i & 1
            self.canvas.itemconfigure(r, outline="#e0461e" if on else "#5f6370", width=5 if on else 3)

        self.fps_n += 1
        now = time.monotonic()
        if now - self.fps_t >= 1:
            self.fps, self.fps_t, self.fps_n = self.fps_n / (now - self.fps_t), now, 0
        msg = f"  ·  {self.message}" if now < self.message_until else ""
        self.canvas.itemconfigure(
            self.status, text=f"{self.fw.name} firmware  ·  {self.fps:4.1f} fps  ·  brightness {b:3d}"
                              f"{'  ·  charging' if self.charging else ''}"
                              f"{f'  ·  web: localhost:{WEB_PORT}' if self.web else ''}{msg}")

        self.next += self.period
        delay = self.next - time.monotonic()
        if delay < -.25:                 # fell behind: don't try to catch up
            self.next, delay = time.monotonic(), 0
        self.root.after(max(1, int(delay * 1000)), self.tick)


def main(args):
    if "-h" in args or "--help" in args:
        print(__doc__)
        return
    names = [a for a in args if not a.startswith("--")]
    fw = Firmware(find_lib(names[0] if names else None), fresh="--fresh" in args)
    print(f"{fw.name} firmware, {fw.fps} fps", flush=True)
    web = None
    if "--web" in args:
        if not fw.has_usb:
            sys.exit(f"--web: the {fw.name} firmware library has no emu_usb (build it again?)")
        web = WebBridge()
        url = f"http://localhost:{WEB_PORT}/"
        print(f"setup page: {url}", flush=True)
        if "--no-browser" not in args:
            webbrowser.open(url)
    ir = None
    if fw.has_ir and "--no-ir" not in args:
        try:
            ir = IrRoom()
            print(f"infrared: sharing {IR_ADDR[0]}:{IR_ADDR[1]} with the other emulators", flush=True)
        except OSError as e:
            print(f"infrared: off ({e})", flush=True)
    console = StdinConsole() if fw.has_console else None
    App(fw, web, ir, console).root.mainloop()


if __name__ == "__main__":
    main(sys.argv[1:])
