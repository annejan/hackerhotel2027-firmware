# C port

Same firmware as the Rust one in `../src`: LED map, LEDPWM driver (from
the reference manual, same as Rust), the buttons, CHRG, the menu, the name tag with name entry (kept in
flash), the hotel screen, the animations, all six games, the ◀ + ▶
reboot into the USB boot ROM, a USB device for the WebUSB setup page
(`src/usb.c`, `src/proto.c`), infrared name swapping with a UART log
and console (`src/ir.c`, `src/irproto.c`, `src/uart.c`, `src/console.c`),
a bring-up test for new boards (`src/selftest.c`), the factory test a
freshly flashed badge runs at the flashing table (`src/factory.c`) and
the sponsor reel on the power-on after it (`src/sponsors.c`). The Rust
firmware has all of these too.
It is plain C on registers, with no WCH peripheral library and no libc. It runs as one
super-loop scheduled on a free-running SysTick. Interrupts: the LED
scanner at the end of each row, and for infrared the SysTick compare
while sending and the receiver's edges.

```sh
make            # fetches WCH's SDK (startup, linker script, headers) on first run
make NAME="ANNE JAN"   # the name tag text
make size
make flash      # wchisp over USB (see ../README.md, Flashing)
```

Needs a RISC-V GCC with rv32 libgcc; the default is the xPack
`riscv-none-elf-gcc` (`CROSS=riscv-none-elf-`). `src/generated.{h,c}` come
from `../tools/gen_layout.py`, which writes the Rust and C tables together.

## Size compared with Rust

Same features in both (the bring-up test is 2.8 KB in C, plus 0.75 KB for
the slot-to-designator table; the factory test 2.4 KB, the sponsor reel
2.2 KB with its five logos).

| | Flash | Static RAM |
|---|---|---|
| C, with USB, IR, the UART console, the bring-up and factory tests and the sponsor reel | 36.1 KB | 6.5 KB (+ 2 KB stack) |
| Rust + Embassy, `opt-level = "z"` (the default) | 49.6 KB | 4.7 KB (+ stack) |
| Rust + Embassy, `opt-level = "s"` | 54.8 KB | 4.7 KB (+ stack) |

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
| app (menu, greeting) | 3930 | 120 |
| generated (LED tables, slot designators) | 3014 | 0 |
| selftest (bring-up test) | 2834 | 44 |
| games/startris | 2556 | 0 |
| factory (factory test) | 2440 | 37 |
| sponsors (sponsor reel, logos) | 2173 | 12 |
| games/snake | 1788 | 688 |
| usb (driver, descriptors) | 1737 | 326 |
| games/pong | 1290 | 0 |
| frame (text, digits, MASK) | 1277 | 448 |
| nametag | 1257 | 0 |
| games/chase | 1230 | 0 |
| game (shell) | 1228 | 516 |
| console | 1172 | 0 |
| games/breakout | 1102 | 0 |
| ir (send, receive) | 1058 | 223 |
| editor (EDIT NAME) | 850 | 40 |
| games/flappy | 718 | 0 |
| irproto (packets, pulse decoder) | 698 | 0 |
| store (name and stage in flash) | 664 | 0 |
| main | 631 | 0 |
| buttons | 574 | 25 |
| proto (USB commands) | 530 | 0 |
| uart (log, line input) | 524 | 566 |
| ledpwm (scanner interrupt, row buffers) | 520 | 1031 |
