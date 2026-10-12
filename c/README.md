# C port

Same firmware as the Rust one in `../src`: LED map, LEDPWM driver (same
guesses), the buttons, CHRG, the menu, the name tag with name entry (kept in
flash), the hotel screen, the animations, all six games, the ◀ + ▶
reboot into the USB boot ROM, a USB device for the WebUSB setup page
(`src/usb.c`, `src/proto.c`), and infrared name swapping with a UART log
and console (`src/ir.c`, `src/irproto.c`, `src/uart.c`, `src/console.c`);
those last two features are C only so far.
It is plain C on registers, with no WCH peripheral library and no libc. It runs as one
super-loop scheduled on a free-running SysTick. The only interrupts are
for infrared: the SysTick compare while sending, and the receiver's edges.

```sh
make            # fetches WCH's SDK (startup, linker script, headers) on first run
make NAME="ANNE JAN"   # the name tag text
make size
make flash      # wchisp over USB (see ../README.md, Flashing)
```

Needs a RISC-V GCC with rv32 libgcc; the default is the xPack
`riscv-none-elf-gcc` (`CROSS=riscv-none-elf-`). `src/generated.{h,c}` come
from `../tools/gen_layout.py`, which writes the Rust and C tables together.

## Size compared with Rust (same features)

| | Flash | Static RAM |
|---|---|---|
| C, GCC 15 `-Os` (xPack), without USB | 21.8 KB | 4.7 KB (+ 2 KB stack) |
| C, with the USB device | 24.2 KB | 5.0 KB (+ 2 KB stack) |
| C, with USB, IR and the UART console | 28.3 KB | 5.8 KB (+ 2 KB stack) |
| Rust + Embassy, `opt-level = "s"` | 35.0 KB | 2.5 KB (+ stack) |
| Rust + Embassy, `opt-level = "z"` | 31.6 KB | 2.5 KB (+ stack) |

About 3 KB of the difference is the Embassy executor, time driver and HAL
interrupt handlers, which the super-loop does not need. The rest is code
size: the same code compiles about 1.5 times larger in Rust.

C keeps its larger buffers static (Snake's demo path search, 688 bytes;
the shared game-state union, 500 bytes), where Rust has them on the stack
or inside the main task.

Where the C flash goes (object sizes, before unused code is dropped):

| Module | Code | Static RAM |
|---|---|---|
| anim (7 animations) | 3960 | 442 |
| app (menu, greeting) | 3378 | 112 |
| games/startris | 2556 | 0 |
| generated (LED tables) | 2246 | 0 |
| games/snake | 1788 | 688 |
| usb (driver, descriptors) | 1737 | 326 |
| games/pong | 1290 | 0 |
| frame (text, digits, MASK) | 1277 | 448 |
| nametag | 1257 | 0 |
| games/chase | 1230 | 0 |
| game (shell) | 1228 | 516 |
| games/breakout | 1102 | 0 |
| ir (send, receive) | 1080 | 236 |
| console | 942 | 0 |
| editor (EDIT NAME) | 850 | 40 |
| games/flappy | 718 | 0 |
| irproto (packets, pulse decoder) | 698 | 0 |
| main | 543 | 0 |
| proto (USB commands) | 530 | 0 |
| uart (log, line input) | 524 | 566 |
| buttons | 522 | 25 |
| store (name in flash) | 506 | 0 |
| ledpwm | 202 | 384 |
