# hh27-starfish firmware (theoretical)

Bare-metal firmware for the Hacker Hotel 2027 starfish badge, in Rust
(`src/`) and in C (`c/`): the same firmware twice, both on the registers
with no HAL. It compiles, but it has never run on a board.

| | | | | |
|---|---|---|---|---|
| ![hotel](docs/fw/hotel.gif) | ![startris](docs/fw/startris.gif) | ![breakout](docs/fw/breakout.gif) | ![star chase](docs/fw/chase.gif) | ![name swap](docs/fw/swap.gif) |
| ![whack-a-mole](docs/fw/whack.gif) | ![simon](docs/fw/simon.gif) | ![maze](docs/fw/maze.gif) | ![higher lower](docs/fw/hilo.gif) | ![challenge progress](docs/fw/progress.gif) |
| ![victory dance](docs/fw/victory.gif) | ![factory test](docs/fw/factory.gif) | ![sponsor reel](docs/fw/sponsors.gif) | ![bring-up test](docs/fw/selftest.gif) | ![4 in a row](docs/fw/four.gif) |
| ![ripple](docs/anim/ripple.gif) | ![galaxy](docs/anim/galaxy.gif) | ![fire](docs/anim/fire.gif) | ![heartbeat](docs/anim/heartbeat.gif) | |

Top three rows: recorded from the firmware itself, running on the PC
(`tools/fwsim`; the name swap from two C emulators), drawn on a KiCad
render of the board with the real LED positions: games and screens, the
challenge games and the progress on the name tag, the victory dance, the
factory test, the sponsor reel, the bring-up test and 4 in a row
against the badge. Bottom row: the
Python animation sketches the firmware ports.

## Using the badge

**[docs/USAGE.md](docs/USAGE.md)** is the full guide: the buttons, the menu,
every game, the name tag and the emulator. The short version:

| | |
|---|---|
| **Hold A** | open the menu (also mid-game; [where it does not](docs/USAGE.md#always-available)) |
| Menu | ◀ / ▶ previous / next, **A** select, hold A close |
| Name, hotel, animations | ◀ / ▶ previous / next screen, A brightness |
| Games | play themselves until you press ◀, ▶ or B |
| **Challenges** | five games, one per arm, five levels each; the name tag's arms show your progress, and all 25 earn the victory dance ([details](docs/USAGE.md#challenges)) |
| **Hold ◀ + ▶, 3 s** | reboot into the USB boot ROM for flashing |
| **B** on the name tag | send your name over infrared; a badge that gets it says HI with your name and sends its own back |
| **4 IN A ROW** | against another badge over infrared (open it on both and point them at each other), or against the badge ([details](docs/USAGE.md#4-in-a-row-against-another-badge)) |
| **First power-on** | a freshly flashed badge runs a 30 s pass / fail factory test for the flashing table, and the power-on after it passed plays the sponsor reel once ([details](docs/USAGE.md#factory-test-the-flashing-table)) |
| **Hold A + B, 3 s** | bring-up test for a new board: COM / PWM / LED walks with designators on the UART, scan rate, IR loopback, power ([details](docs/USAGE.md#bring-up-test-first-boards)) |

◀ and ▶ are the two buttons on the front of the lower legs (thumbs), A and
B the two on the back of the side arms (A right, B left).

Type your name on the badge with **EDIT NAME** in the menu (◀/▶ letter,
A next, hold A save). It is kept in flash. Until then the badge shows the
build-time default: `BADGE_NAME="ANNE JAN" cargo build --release` (C:
`make -C c NAME="ANNE JAN"`).

## Setup page (WebUSB)

Both firmwares are a USB device with a small vendor protocol, and
[`docs/webusb/index.html`](docs/webusb/index.html) is a page that talks to it
from Chrome or Edge with WebUSB: set your name (saved in flash), pick what
the badge shows, set the brightness. It also shows the badge's state as it
changes. No driver install is needed on Linux and macOS; on Windows the
badge says so itself (Microsoft OS 2.0 descriptor, WinUSB).

Try it without a badge: `python3 tools/emu/emu.py c --web` (or `rust`) serves the page
on <http://localhost:8027/> and passes its packets to the emulated firmware,
so the page, the protocol and the badge's reaction are tested together.

The USB driver (`c/src/usb.c`, and `src/usb.rs` with byte-identical
descriptors) is ported from the ch32fun-based WebUSB test that the badge
team ran on a CH32X035, where it enumerated and passed bulk data. The
CH643's USB block has the same address and registers, but the port is
**not tested on a CH643 yet**. The protocol is in `c/src/proto.h` /
`src/proto.rs`; INFO says which firmware answers (`c` or `r`).
It uses the pid.codes test ID 1209:D035 for now; a real badge needs its own.

## Infrared and the UART console

**Name swap.** B on the name tag sends your name over infrared. A badge
that receives it shows "HI" and your name, and answers with its own name,
which yours then shows: one press, and both know who they met. The format
is NEC-style pulse-distance coding on 38 kHz, with a type, a length and a
CRC (`c/src/irproto.h`, `src/irproto.rs`). TIM2 CH1 makes the carrier on
PB21, the LED's cathode, in both (low = LED on); TIM2 is an advanced
timer on the CH643, so it needs BDTR.MOE set before anything comes out.
The marks and spaces are timed by the SysTick compare interrupt in C
(`c/src/ir.c`) and by TIM3 in one-shot mode in Rust (`src/ir.rs`). Both
receive the same way: an EXTI interrupt on both edges of PB19 (line 19)
times each mark and space with SysTick, and the main loop decodes them.
The C driver is tested on the PC against a fake SysTick, TIM2, EXTI and GPIO (`c/test/test_ir.c`:
waveform out decodes, distorted edges in decode, own echo ignored, a send
whose deadline passed during a flash write still ends), the Rust decoder
with a distorted, edge-timed packet; neither is tested on a badge.
C and Rust badges swap names with each other (checked in the emulator).

**UART console.** The debug header J3 (115200 8N1) prints a log (boot,
names saved, IR packets) and takes commands: `help`, `info`,
`name TEXT`, `show N`, `level N`, `hello`, `test`, `factory`,
`sponsors`, `progress`.

Both work in the emulator: every emulator on a PC shares one infrared
"room" (UDP broadcast on 127.255.255.255:8028), so start two and press B
on one; stdin is the console and the log goes to stderr.

## Emulator

Either firmware can be built as a shared library and played on the PC:

```sh
make -C c emu && python3 tools/emu/emu.py c                     # C firmware
(cd tools/fwsim && cargo build --release) && python3 tools/emu/emu.py rust
```

The arrow keys are ◀ ▶ A B (← → ↑ ↓), and the buttons are drawn on the
board where they are; see
[docs/USAGE.md](docs/USAGE.md#emulator).

## Hardware

| | |
|---|---|
| MCU | WCH **CH643Q**: QingKe V4C RISC-V (RV32IMAC), 48 MHz, 62 KiB flash, 20 KiB RAM, LQFP64 |
| LEDs | 384 slots in a 24 × 16 matrix: PA0–PA23 drive the anodes (PWM), PB0–PB15 sink the cathodes (COM) |
| | 208 mono LEDs in a 17 × 16 grid, 71 mono LEDs on the outline, 35 RGB LEDs (7 per arm) |
| Buttons | ◀ SW3 PC4, B SW1 PB20, A SW2 PC5, ▶ SW4 PB18; active low, 10k pull-ups on the board |
| Power | LiPo (JST J2) with XT4054 charger and power path; CHRG on PB22; VBUS sense on PC3 |
| Debug | header J3, 1 × 6 at 1.27 mm: 1 RX (PB17), 2 TX (PB16), 3 SWIO (PC18), 4 SWCK (PC19), 5 VCC, 6 GND. USART3 for the log and console, and SWD |
| IR | both on the front: receiver CGQ1 (38 kHz, on VCC) below the centre, on PB19; IR LED D316 above the centre, anode on VCC through R37 (200 Ω, about 15 mA), cathode PB21 (low = on). Name swap |

## Build

```sh
cargo build --release            # Rust 1.98.0 (rust-toolchain.toml), target riscv32imc-unknown-none-elf
cargo run --release              # flashes over USB with wchisp, see Flashing
```

`src/generated.rs` contains the LED map. It is generated from the KiCad
board, so run this again after layout changes:

```sh
python3 tools/gen_layout.py      # needs KiCad's pcbnew Python module
```

The tools look for the board in a
[hackerhotel2027-hardware](https://github.com/badgeteam/hackerhotel2027-hardware)
checkout next to this repo, or in `$HH27_HW`.

The script follows every LED anode through its series resistor to a PA pin
and every cathode to a PB pin. It fails if two LEDs share one slot.

## Flashing

There are two ways in. With only a USB-C cable, through the CH643's
factory boot ROM (3328 bytes of system flash) and
[wchisp](https://github.com/ch32-rs/wchisp) (`cargo install wchisp`), as
below. Or with a WCH-LinkE on the debug header J3 (3 SWIO, 4 SWCK, 5 VCC,
6 GND), which also gives a debugger. Pin 5 of J3 is the badge's VCC
(battery or USB, up to about 5 V), not 3.3 V.
wchisp lists the CH643 with USB and UART ISP support (`devices/0x24-CH643.yaml`).
USB-C D+/D− go straight to PC17/PC16, the CH643's USB pins.

```sh
wchisp probe
wchisp flash target/riscv32imc-unknown-none-elf/release/hh27-starfish
```

`wchisp flash` erases whole 1 KiB sectors, one more than the image
needs. Below 61440 bytes that leaves the last two pages (the name, the
challenges' progress and the first-boot stage) alone; from 61440 bytes on,
every update would wipe them. CI fails either firmware when it gets there.

On Linux, give your user access to the boot ROM's USB ID (4348:55e0), for
example with a udev rule:
`SUBSYSTEM=="usb", ATTRS{idVendor}=="4348", ATTRS{idProduct}=="55e0", MODE="0666"`.

How the chip gets into the boot ROM (none of this is tested on a CH643 yet):

- **Blank chip**: on the CH32X035 the boot ROM always runs when the
  application flash is empty. The CH32X035 has the same core and memory map
  as the CH643 (ch32-data uses one memory file for both). If the CH643 does
  the same, new badges can be flashed straight over USB.
- **From firmware**: hold ◀ + ▶ for 3 s. The firmware does what WCH's
  `SystemReset_StartMode(Start_Mode_BOOT)` does (set FLASH_STATR.BOOT_MODE
  after the BOOT_MODEKEYR keys) and resets; the boot ROM then starts.
  See `src/bootloader.rs` / `c/src/bootloader.c`.
- **Pins**: on the CH32X035 the boot ROM runs when PC16 is low and PC17 is
  high at power-on. The badge has nothing that pulls PC17 high; with the
  SWD header that is no longer needed.

A firmware that crashes before it can jump to the boot ROM no longer bricks
the badge: the SWD header gets it back with a WCH-LinkE.

## Structure

| File | What |
|---|---|
| `main.rs` | Superloop on a free-running SysTick, like `c/src/main.c`: buttons every 10 ms, CHRG every 500 ms, IR decode every 5 ms, frames at 30 fps, USB and UART on every pass; an 8-entry event queue; clock setup, unused pads |
| `buttons.rs` | Button polling: debounce, press / tap / hold events, held buttons, the ◀ + ▶ chord |
| `app.rs` | Menu, the list of screens, brightness, charging overlay |
| `nametag.rs` | Name tag and Hacker Hotel 2027 screens, the `Name` type |
| `editor.rs` | Name entry on the badge (EDIT NAME) |
| `store.rs` | The name, the challenges' progress and the first-boot stage in flash (the last two 256-byte pages, written in turns so a power cut during a save loses nothing; `memory.x` keeps code out of them) |
| `anim.rs` | Animations: ripple, galaxy, heartbeat, life, fire, twinkle, charging |
| `game.rs` | Game shell: demo mode, play, crash, score screen |
| `games/` | Snake, Startris, Breakout, Pong, Flappy, Star Chase, Whack-a-mole, Simon, Maze, Higher lower, 4 in a row (also between two badges over infrared) |
| `challenge.rs` | The five challenges (levels, progress on the arms) and the victory dance |
| `frame.rs` | Frame buffer, text and digits, integer HSV, sine, RNG |
| `font.rs` | 5x7 font (` `..`Z`), 3x5 digits |
| `ledpwm.rs` | CH643 LEDPWM driver, from the reference manual: one interrupt per row, double-buffered frames |
| `bootloader.rs` | Reboot into the USB boot ROM |
| `generated.rs` | LED map, polar coordinates, gamma and sine tables (generated) |
| `usb.rs`, `proto.rs` | USB device on the registers (WebUSB setup page), and its protocol |
| `ir.rs`, `irproto.rs` | Infrared: TIM2 CH1 carrier on PB21 (with MOE), TIM3 mark and space timing, receive on EXTI line 19; packets and pulse decoder |
| `uart.rs`, `console.rs`, `log.rs` | UART header: the log out, console commands in (received on the USART3 interrupt) |
| `selftest.rs` | Bring-up test for new boards |
| `factory.rs` | Factory test for the flashing table (first power-on until it passes) |
| `sponsors.rs`, `logos.rs` | Sponsor reel (the power-on after the factory test), with the sponsor table; the logos (generated) |

`c/` has a port of the same firmware to plain C (register level, no WCH
peripheral library), for comparison; see `c/README.md` and the sizes below.

## Tools

| | |
|---|---|
| `tools/emu` | Live emulator for either firmware, built as a shared library (see Emulator); `--web` serves the setup page against it (and opens it in the browser, unless `--no-browser`); `--fresh` starts as a freshly flashed badge (factory test, then sponsor reel); emulators share infrared, stdin is the UART console |
| `tools/fwsim` | `cargo test` runs 19 tests: the name editor, label scrolling, browsing, name clean-up, console commands, the USB protocol (also fuzzed), the IR decoder on edge-timed pulses, the challenge levels, the four challenge games played to level 5, and 4 in a row's rules and play. Runs the firmware's hardware-independent modules (app, menu, games, animations) on the PC, from the same sources. `soak` plays every item with random presses in a debug build, where integer overflow and out-of-range indexing panic; `record` writes frames for `fwgif.py`. Also builds the Rust library for the emulator. Run it from its own directory: `cd tools/fwsim && cargo run -- soak` |
| `tools/emu/parity.py` | Runs the C and Rust emulator libraries side by side and compares them frame for frame, on the screens that use no random numbers |
| `tools/emu/ir_game.py` | Two emulated badges play 4 in a row over infrared, for every pairing of C and Rust, also over a link that loses 30% of the packets; both must pair up and agree on every move and the result |
| `c/test` | `make -C c test`: C host tests under ASan and UBSan. The drivers (buttons, ir, ledpwm, store) against fake registers, the app through its interface (names, editor, infrared, console, factory test, USB, browsing), the challenges played to level 5, 4 in a row (rules, the badge's play, the infrared side), and a 200 000-frame soak |
| `.github/workflows/ci.yml` | CI: builds both firmwares and fails when either is over 61440 bytes of flash (see Flashing), runs the C tests, the fwsim tests and soak, the parity check and the infrared game |
| `tools/sim/logo.py` | Sponsor logos: `assets/sponsors/*.svg` to greyscale bitmaps for the grid (`c/src/logos.h`, `src/logos.rs`); needs Inkscape and scipy |
| `tools/sim/fwgif.py` | Turns a `fwsim record` (or frames from the C emulator library) into a GIF on the board render |
| `tools/sim/hh2027.py` | Animation sketches on a KiCad 3D render of the board (GIF + MP4 into `previews/`); `--refresh` re-reads the board (kicad-cli + pcbnew) |
| `tools/sim/games.py` | The first game mock-ups, played by a scripted AI |
| `tools/gen_layout.py` | LED map from the KiCad board, for Rust and C |

The Python tools need numpy, scipy and Pillow; the emulator also needs
tkinter, and the GIF and MP4 output (`hh2027.py`, `fwgif.py`, `games.py`)
ffmpeg.

## Size

Everything above (name tag and name entry, hotel screen, menu, 6
animations, 11 games, the challenges and the victory dance, the USB boot
gesture, the USB device, the infrared name swap and 4 in a row, the UART
console, the bring-up test, the factory test and the sponsor reel),
measured. The last 512 bytes of flash are kept for the saved name, the
challenges' progress and the first-boot stage.

| | Flash (of 61.5 KB) | Static RAM (of 20 KB) |
|---|---|---|
| C, GCC 15 `-Os` | **47.3 KB** (77%) | 4.7 KB + 2 KB stack |
| Rust, `opt-level = "z"` (default) | **57.0 KB** (93%) | 2.2 KB + stack |
| Rust, `opt-level = "s"` | does not fit any more | |

From `size`: flash is text + data, static RAM data + bss (C's bss
includes its 2 KB stack, counted separately here). The Rust stack is not
static: main's stack also holds the App, about 1.3 KB.

Rust has 4.5 KB left, C 14.2 KB; below the 61440-byte point where
`wchisp flash` would wipe the settings (see Flashing), 3.0 and 12.7 KB.
4 in a row costs 2.9 KB in C and about 4.5 KB in Rust (with the packet
type the infrared queue now uses).
The challenges (four games, the progress and the victory dance) cost
7.6 KB in C (the dance with its stick figure and fireworks 2.9 KB, the
maze 1.8 KB) and about 8.8 KB in Rust. The factory test and the sponsor
reel cost 4.9 KB in C (the test 2.4 KB, a third of it the volunteers'
hints in the log; the reel 2.2 KB, of which the five greyscale logos are
0.94 KB) and about 5.3 KB in Rust (the Rust figures were measured when it
still ran on Embassy).

Roughly where the C flash goes: animations 4.0 KB, the app (menu, USB
settings, infrared greeting) 4.3 KB, Startris 2.5 KB, LED tables 2.2 KB,
Snake 1.8 KB (its demo path search included),
Pong / Star Chase / game shell / name tag / Breakout about 1.1 to 1.3 KB
each, Flappy 0.7 KB. The 5x7 font is 295 bytes.

USB in C is 2.3 KB: the driver about 1.25 KB, descriptors 0.3 KB (the
Windows part 195 bytes), the protocol 0.4 KB, and 326 bytes of RAM.
Infrared and the UART console add 4.8 KB (IR driver 1.1 KB, pulse codec
0.7 KB, UART 0.6 KB, console 1.4 KB, the greeting in the app about 1 KB)
and 0.9 KB of RAM, mostly the UART's 512-byte send and 128-byte receive
buffers.

Rust is 9.7 KB (9886 bytes, 1.20 times) larger than C for the same
features, with no runtime left to account for it: the gap is code
generation in the app code (panic paths, bounds checks, inlining).
Until the bare-metal rewrite the Rust firmware ran on Embassy with
ch32-hal, and was 58.5 KB. Embassy served only as a timer service (six
tasks, each a poll and a sleep, and one channel; no peripheral was
awaited), at about 5.6 KB of flash and 2.7 KB of RAM. ch32-hal's own EXTI
handlers kept IR receive off EXTI, so it sampled the receiver on TIM1
instead, and ch32-hal's CH643 support came from git only, as it is not on
crates.io. Now the Rust firmware uses ch32-metapac, qingke, qingke-rt and
critical-section (34 crates in `Cargo.lock`, all from crates.io).

## What is solid and what is guessed

**Solid**

- ch32-metapac 0.1.1 on crates.io has a `ch643` feature: the register
  definitions the Rust firmware uses. Clock setup does what ch32-hal's
  CH643 init does (HSI, two flash wait states, HCLK = SYSCLK).
- The LED map is traced from the PCB nets.

- **LEDPWM**, from the CH643 reference manual (CH643RM V1.9, chapter 24).
  The board is its "8 columns RGB, built-in drive" case; reference circuit
  24.4.2 is 16 rows x 8 RGB columns, as here. MATRIX_MODE PA24, PA pins
  active high, PB COM lines active low, one interrupt per row in which
  the driver DMA-loads the next row (8 words: intensity, PA k, PA k+8,
  PA k+16) and selects the next COM. The brightness is the intensity PWM.
- **Remaps** (RM 8.3.2.1): USART3_RM = 11 puts TX/RX on PB16/PB17, and
  TIM2_RM = 001 puts CH1 on PB21, which makes the IR carrier (its CH2..CH4
  land on LED matrix pins then, so they stay off). PB19, the receiver, has
  only TIM2 CH4 (TIM2_RM = 100) as a timer input, so both firmwares read
  it on EXTI line 19. (ch32-data's CH643 tables stop at USART3 remap 2
  and TIM1 remap 3.)
- **SysTick** (QingKe V4 manual, chapter 5): with STRE = 0 the counter runs
  on past the compare value and sets CNTIF there, as `ir.c` uses it.

**Guessed** (each place is marked `GUESS` in the source)

- **LEDPWM details** the manual leaves open: that the LED reference clock is
  HCLK (it sets the 183 Hz scan rate), that a 6-bit intensity uses the low
  6 bits of the intensity byte, and that the DMA address is the low 16
  bits of a RAM address. All in `ledpwm.rs` / `c/src/ledpwm.c`.

## Hardware notes found along the way

What to check, in order, on the first prototype: **[docs/BRINGUP.md](docs/BRINGUP.md)**.

- **Unbonded and unused pads.** WCH's SystemInit pulls up the pads that the
  LQFP64 package does not bond out (PC6–PC9, PC12, PC13, PC20); if they
  float they draw current. The firmware does it too, on the registers:
  `pull_up_unused_pads` in `main.rs`, and `c/src/quirks.c` in place of
  WCH's `GPIO_IPD_Unused`. Both also pull up PC0–PC2, PC14 and PC15,
  which the board leaves unconnected (the touch pads and the CC jumpers
  went); `ir.rs` / `ir.c` pull up PB23 (no longer the IR LED). PC3 (VBUS
  sense) must stay a floating input.
- **IR receiver supply: fixed** in hardware commit 0986add. Its VCC
  (through R36) and the IR_RECV pull-up R35 were on a `+3.3V` net that
  nothing drove; they are on VCC now.
- **VBUS straight into PC3.** VCC is VBUS minus a Schottky drop, so PC3
  sits a little above VDD while USB is in; the datasheet allows at most
  VDD + 0.3 V on an I/O pin (CH643DS0 3.2). A divider would be safer.
- **IR LED current: fixed** in hardware commit 0986add. The anode hung
  off PB23 through 47 Ω, more than a PB pin may source (30 mA absolute
  maximum). Now the anode is on VCC through R37 (200 Ω, about 15 mA) and
  PB21 sinks the cathode, with the carrier from TIM2 CH1.
- **Wiring order is not the order you see.** Grid column 7 (D143) goes
  through R10 to PA9, columns 8 and 9 (D141, D142) through R8 and R9 to
  PA7 and PA8; grid row 7 is on PB15, after row 15 on PB0. Harmless: the
  LED map (`tools/gen_layout.py`) puts every LED where it is, so text and
  games are unaffected. Only a test that walks the wires shows it; the
  factory test walks the grid in the order you see instead.
- **No battery gauge.** The board has no battery-voltage sense, so the
  firmware can show *that* the battery charges but not *how full* it is.
  Asked for: VBAT through a divider (2 × 1 MΩ, 100 nF on the middle) to
  PC0 (ADC channel A10), which is free.
- **USB on the registers.** `src/usb.rs` drives the USB block on its
  registers, as `c/src/usb.c` does (ch32-hal, which the Rust firmware
  used before, had no USB driver for the CH643); both are polled on every
  pass of the main loop. Both always use the 5 V PHY setup (its own 3.3 V
  regulator, 10k on D+): USB only works when VDD comes from VBUS, so
  choosing by VDD at boot would go wrong for a badge started on its
  battery and plugged in later.
