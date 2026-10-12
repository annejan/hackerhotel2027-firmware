# C port

Same firmware as the Rust one in `../src`: LED map, LEDPWM driver (same
guesses), TouchKey, CHRG, the seven animations, Snake and the T1 + T4
reboot into the USB boot ROM. It is plain C on
registers, with no WCH peripheral library and no libc. It runs as one
super-loop scheduled on a free-running SysTick, with no interrupts.

```sh
make            # fetches WCH's SDK (startup, linker script, headers) on first run
make size
make flash      # wchisp over USB (see ../README.md, Flashing)
```

Needs a RISC-V GCC with rv32 libgcc; the default is the xPack
`riscv-none-elf-gcc` (`CROSS=riscv-none-elf-`). `src/generated.{h,c}` come
from `../tools/gen_layout.py`, which writes the Rust and C tables together.

## Size compared with Rust (same features)

| | Flash | Static RAM |
|---|---|---|
| C, GCC 15 `-Os` (xPack) | 11.1 KB | 4.6 KB (+ 2 KB stack) |
| Rust + Embassy, `opt-level = "s"` | 19.8 KB | 2.4 KB (+ stack) |
| Rust + Embassy, `opt-level = "z"` | 18.7 KB | 2.4 KB (+ stack) |

About 3 KB of the difference is the Embassy executor, time driver and HAL
interrupt handlers, which the super-loop does not need. The rest is code
size: the same code compiles about 1.5 to 2 times larger in Rust.

Snake alone (game, attract-mode autopilot, score screen) costs 2.5 KB in C
and 3.8 KB in Rust. Its RAM: a 416-byte body ring buffer, plus about 700
bytes for the autopilot's path search (static in C, on the stack in Rust).
