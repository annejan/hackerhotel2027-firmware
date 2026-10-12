# C port

Same firmware as the Rust one in `../src`: LED map, LEDPWM driver (same
guesses), TouchKey, CHRG and the seven animations. It is plain C on
registers, with no WCH peripheral library and no libc. It runs as one
super-loop scheduled on a free-running SysTick, with no interrupts.

```sh
make            # fetches WCH's SDK (startup, linker script, headers) on first run
make size
make flash      # wlink + WCH-LinkE
```

Needs a RISC-V GCC (`CROSS=riscv64-elf-` by default; any rv32-capable one
works). `src/generated.{h,c}` come from `../tools/gen_layout.py`, which
writes the Rust and C tables together.

## Size compared with Rust (same features)

| | Flash | Static RAM |
|---|---|---|
| C, GCC 15 `-Os` | 8.1 KB | 3.4 KB (+ 2 KB stack) |
| Rust + Embassy, `opt-level = "s"` | 15.2 KB | 1.8 KB (+ stack) |
| Rust + Embassy, `opt-level = "z"` | 14.4 KB | 1.8 KB (+ stack) |

The difference is about 7 KB. About 3 KB of it is the Embassy executor,
time driver and HAL interrupt handlers, which the super-loop does not need.
The rest is code size: the same animations compile to about 6.7 KB in Rust
and about 3.5 KB in C.
