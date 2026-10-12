# C port

Same firmware as the Rust one in `../src`: LED map, LEDPWM driver (same
guesses), TouchKey, CHRG, the menu, name tag and hotel screens, the
animations, all six games and the T1 + T4 reboot into the USB boot ROM.
It is plain C on registers, with no WCH peripheral library and no libc. It runs as one
super-loop scheduled on a free-running SysTick, with no interrupts.

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
| C, GCC 15 `-Os` (xPack) | 20.3 KB | 4.7 KB (+ 2 KB stack) |
| Rust + Embassy, `opt-level = "s"` | 31.8 KB | 2.4 KB (+ stack) |
| Rust + Embassy, `opt-level = "z"` | 28.8 KB | 2.4 KB (+ stack) |

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
| games/startris | 2542 | 0 |
| generated (LED tables) | 2246 | 0 |
| games/snake | 1788 | 688 |
| app (menu) | 1499 | 20 |
| games/pong | 1290 | 0 |
| frame (text, digits, MASK) | 1277 | 448 |
| games/chase | 1230 | 0 |
| game (shell) | 1228 | 516 |
| nametag | 1143 | 0 |
| games/breakout | 1102 | 0 |
| games/flappy | 718 | 0 |
| touch | 636 | 49 |
