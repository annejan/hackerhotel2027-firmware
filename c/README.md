# C port

Same firmware as the Rust one in `../src`: LED map, LEDPWM driver (same
guesses), TouchKey, CHRG, the seven animations and the T1 + T4 reboot into
the USB boot ROM. It is plain C on
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
| C, GCC 15 `-Os` (xPack) | 8.5 KB | 3.4 KB (+ 2 KB stack) |
| Rust + Embassy, `opt-level = "s"` | 16.0 KB | 1.9 KB (+ stack) |
| Rust + Embassy, `opt-level = "z"` | 15.1 KB | 1.9 KB (+ stack) |

The difference is about 7.5 KB. About 3 KB of it is the Embassy executor,
time driver and HAL interrupt handlers, which the super-loop does not need.
The rest is code size: the same animations compile to about 6.7 KB in Rust
and about 3.5 KB in C.
