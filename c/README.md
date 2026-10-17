# C port

Same firmware as the Rust one in `../src`: LED map, LEDPWM driver (from
the reference manual, same as Rust), the buttons, CHRG, the menu, the name tag with name entry (kept in
flash), the hotel screen, the animations, all ten games, the ◀ + ▶
reboot into the USB boot ROM, a USB device for the WebUSB setup page
(`src/usb.c`, `src/proto.c`), infrared name swapping with a UART log
and console (`src/ir.c`, `src/irproto.c`, `src/uart.c`, `src/console.c`),
a bring-up test for new boards (`src/selftest.c`), the factory test a
freshly flashed badge runs at the flashing table (`src/factory.c`) and
the sponsor reel on the power-on after it (`src/sponsors.c`), and the
five challenges with their victory dance (`src/challenge.c`, four of the
games in `src/games/`). The Rust
firmware has all of these too.
It is plain C on registers, with no WCH peripheral library and no libc. It runs as one
super-loop scheduled on a free-running SysTick. Interrupts: the LED
scanner at the end of each row, the UART's receive (into a ring buffer),
and for infrared the SysTick compare while sending and the receiver's
edges (EXTI line 19).

```sh
make            # fetches WCH's SDK (startup, linker script, headers) on first run
make NAME="ANNE JAN"   # the name tag text
make size
make flash      # wchisp over USB (see ../README.md, Flashing)
make test       # host tests under ASan and UBSan (test/), with the PC's cc
```

Needs a RISC-V GCC with rv32 libgcc; the default is the xPack
`riscv-none-elf-gcc` (`CROSS=riscv-none-elf-`). A `riscv64-elf-gcc`
built without multilib (openSUSE's, for one) has no rv32 libgcc and does
not link. `src/generated.{h,c}` come
from `../tools/gen_layout.py`, which writes the Rust and C tables together.

## Size compared with Rust

Same features in both (the bring-up test is 2.8 KB in C, plus 0.75 KB for
the slot-to-designator table; the factory test 2.4 KB, the sponsor reel
2.2 KB with its five logos, the challenges 7.6 KB with their four games and the dance,
4 in a row 2.9 KB with its infrared play).

| | Flash | Static RAM |
|---|---|---|
| C, with USB, IR, the UART console, the bring-up and factory tests, the sponsor reel, the challenges and 4 in a row | 47.6 KB (48720 B) | 4.7 KB (+ 2 KB stack) |
| Rust, `opt-level = "z"` (the default) | 52.9 KB (54140 B) | 2.2 KB (+ stack) |
| Rust, `opt-level = "s"` | 59.8 KB (61214 B) | |

The Rust firmware is a super-loop on the registers too now, so there is
no runtime in the difference (Embassy and ch32-hal took about 5.6 KB
before; see ../README.md, Size). The 5.3 KB that is left is code
generation: bounds checks, iterators and games copied into place make the
same code about 1.1 times larger in Rust (1.2 before the Rust build got
linker relaxation).

C keeps its larger buffers static (Snake's demo path search, 688 bytes;
the shared game-state union, 500 bytes), where Rust has them on the stack
(the App lives on main's stack, about 1.3 KB).

Where the C flash goes (object sizes, before unused code is dropped):

| Module | Code | Static RAM |
|---|---|---|
| app (menu, greeting, first boot) | 4531 | 124 |
| anim (7 animations) | 3960 | 442 |
| generated (LED tables, slot designators) | 3014 | 0 |
| challenge (levels, progress, victory dance) | 2972 | 31 |
| games/four (4 in a row, also over infrared) | 2921 | 0 |
| selftest (bring-up test) | 2834 | 44 |
| games/startris | 2556 | 0 |
| factory (factory test) | 2440 | 37 |
| sponsors (sponsor reel, logos) | 2173 | 12 |
| games/snake | 1788 | 688 |
| games/maze | 1770 | 0 |
| game (shell) | 1721 | 516 |
| usb (driver, descriptors) | 1665 | 326 |
| console | 1412 | 0 |
| frame (text, digits, MASK) | 1307 | 452 |
| games/pong | 1290 | 0 |
| nametag | 1257 | 0 |
| store (name, progress, stage in two flash pages) | 1240 | 0 |
| games/chase | 1230 | 0 |
| ir (send, receive) | 1164 | 223 |
| games/breakout | 1102 | 0 |
| main | 1001 | 0 |
| games/simon | 929 | 0 |
| editor (EDIT NAME) | 850 | 40 |
| games/hilo | 790 | 0 |
| games/flappy | 718 | 0 |
| games/whack | 714 | 0 |
| irproto (packets, pulse decoder) | 698 | 0 |
| uart (log, line input on the RX interrupt) | 646 | 696 |
| buttons | 574 | 25 |
| ledpwm (scanner interrupt, row buffers) | 538 | 1032 |
| proto (USB commands) | 530 | 0 |
