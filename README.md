# hh27-starfish firmware (theoretical)

Embassy firmware for the Hacker Hotel 2027 starfish badge. It compiles, but it
has never run on a board.

| | | | |
|---|---|---|---|
| ![hotel](docs/fw/hotel.gif) | ![startris](docs/fw/startris.gif) | ![breakout](docs/fw/breakout.gif) | ![star chase](docs/fw/chase.gif) |
| ![ripple](docs/anim/ripple.gif) | ![galaxy](docs/anim/galaxy.gif) | ![fire](docs/anim/fire.gif) | ![heartbeat](docs/anim/heartbeat.gif) |

Top row: recorded from the firmware itself, running on the PC
(`tools/fwsim`), drawn on a KiCad render of the board with the real LED
positions. Bottom row: the Python animation sketches the firmware ports.

## Using the badge

| | |
|---|---|
| **Hold T3** | open the menu (from anywhere, also mid-game) |
| Menu | T1 / T4 previous / next, **T3** select, hold T3 close |
| Name, hotel, animations | T1 / T4 previous / next item, T3 brightness |
| Games | demo mode until you touch T1, T2 or T4; then the game has the pads |
| **Hold T1 + T4, 3 s** | reboot into the USB boot ROM for flashing (an arrow runs down first) |
| USB power | a battery gauge shows while charging, until the next tap |

Menu items: **NAME** (name tag, the start screen), **HACKER HOTEL**,
RIPPLE, GALAXY, HEART, LIFE, FIRE, TWINKLE, **SNAKE**, **STARTRIS**,
**BREAKOUT**, **PONG**, **FLAPPY**, **STAR CHASE**, and LIGHT (brightness,
from night mode to full).

| Game | Pads |
|---|---|
| Snake | T1 turn left, T4 turn right |
| Startris | T1 / T4 move (hold to repeat), T3 rotate, T2 drop |
| Breakout | T1 / T4 move the paddle (hold) |
| Pong (you vs the badge, to 5) | T1 up, T4 down (hold) |
| Flappy | T1, T2 or T4 flap |
| Star Chase | T1, T2 or T4 stop the light beside the green arm |

Startris uses T2 only to drop; the rest works without it (see "T2 may not
work" below).

The name is set at build time: `BADGE_NAME="ANNE JAN" cargo build --release`
(C: `make NAME="ANNE JAN"`). Letters, digits and ` !'*-.:?/#@`.

## Hardware

| | |
|---|---|
| MCU | WCH **CH643Q**: QingKe V4C RISC-V (RV32IMAC), 48 MHz, 62 KiB flash, 20 KiB RAM, LQFP64 |
| LEDs | 384 slots in a 24 × 16 matrix: PA0–PA23 drive the anodes (PWM), PB0–PB15 sink the cathodes (COM) |
| | 208 mono LEDs in a 17 × 16 grid, 71 mono LEDs on the outline, 35 RGB LEDs (7 per arm) |
| Touch | T1–T4 on PC0–PC3, TouchKey channels 10–13 |
| Power | LiPo with XT4054 charger; CHRG on PB22 |
| Debug | SWIO / SWCK go only to the MCU: no pads, so no WCH-LinkE |

## Build

```sh
cargo build --release            # stable Rust, target riscv32imc-unknown-none-elf
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

The board has no debug pads, so the only way in is USB-C, through the
CH643's factory boot ROM (3328 bytes of system flash) and
[wchisp](https://github.com/ch32-rs/wchisp) (`cargo install wchisp`).
wchisp lists the CH643 with USB and UART ISP support (`devices/0x24-CH643.yaml`).
USB-C D+/D− go straight to PC17/PC16, the CH643's USB pins.

```sh
wchisp probe
wchisp flash target/riscv32imc-unknown-none-elf/release/hh27-starfish
```

On Linux, give your user access to the boot ROM's USB ID (4348:55e0), for
example with a udev rule:
`SUBSYSTEM=="usb", ATTRS{idVendor}=="4348", ATTRS{idProduct}=="55e0", MODE="0666"`.

How the chip gets into the boot ROM (none of this is tested on a CH643 yet):

- **Blank chip**: on the CH32X035 the boot ROM always runs when the
  application flash is empty. The CH32X035 has the same core and memory map
  as the CH643 (ch32-data uses one memory file for both). If the CH643 does
  the same, new badges can be flashed straight over USB.
- **From firmware**: hold T1 + T4 for 3 s. The firmware does what WCH's
  `SystemReset_StartMode(Start_Mode_BOOT)` does (set FLASH_STATR.BOOT_MODE
  after the BOOT_MODEKEYR keys) and resets; the boot ROM then starts.
  See `src/bootloader.rs` / `c/src/bootloader.c`.
- **Pins**: on the CH32X035 the boot ROM runs when PC16 is low and PC17 is
  high at power-on. The badge has nothing that pulls PC17 high, so this
  needs a pad or button.

So a firmware that crashes before it can jump to the boot ROM bricks the
badge, unless the chip also enters the boot ROM by itself. Before
production, the hardware should get SWIO + GND test pads, and a pad or
button that pulls PC17 (D+) high at power-on.

## Structure

| File | What |
|---|---|
| `main.rs` | Embassy main: render loop at 30 fps, event channel, CHRG monitor task |
| `touch.rs` | TouchKey polling task: press / tap / hold events, held pads, the T1 + T4 chord |
| `app.rs` | Menu, the list of screens, brightness, charging overlay |
| `nametag.rs` | Name tag and Hacker Hotel 2027 screens |
| `anim.rs` | Animations: ripple, galaxy, heartbeat, life, fire, twinkle, charging |
| `game.rs` | Game shell: demo mode, play, crash, score screen |
| `games/` | Snake, Startris, Breakout, Pong, Flappy, Star Chase |
| `frame.rs` | Frame buffer, text and digits, integer HSV, sine, RNG |
| `font.rs` | 5x7 font (` `..`Z`), 3x5 digits |
| `ledpwm.rs` | CH643 LEDPWM driver: **mostly guesswork, see below** |
| `bootloader.rs` | Reboot into the USB boot ROM |
| `generated.rs` | LED map, polar coordinates, gamma and sine tables (generated) |

`c/` has a port of the same firmware to plain C (register level, no WCH
peripheral library), for comparison; see `c/README.md` and the sizes below.

## Tools

| | |
|---|---|
| `tools/fwsim` | Runs the firmware's hardware-independent modules (app, menu, games, animations) on the PC, from the same sources. `soak` plays every item with random touches in a debug build, where integer overflow and out-of-range indexing panic; `record` writes frames for `fwgif.py`. Run it from its own directory: `cd tools/fwsim && cargo run -- soak` |
| `tools/sim/fwgif.py` | Turns a `fwsim record` into a GIF on the board render |
| `tools/sim/hh2027.py` | Animation sketches on a KiCad 3D render of the board (GIF + MP4 into `previews/`); `--refresh` re-reads the board (kicad-cli + pcbnew) |
| `tools/sim/games.py` | The first game mock-ups, played by a scripted AI |
| `tools/gen_layout.py` | LED map from the KiCad board, for Rust and C |

The Python tools need numpy, scipy, Pillow and ffmpeg.

## Size

Everything above (name tag, hotel screen, menu, 6 animations, 6 games, the
USB boot gesture), measured:

| | Flash (of 62 KB) | Static RAM (of 20 KB) |
|---|---|---|
| C, GCC 15 `-Os` | **20.3 KB** (32%) | 4.7 KB + 2 KB stack |
| Rust + Embassy, `opt-level = "z"` | 28.8 KB (46%) | 2.4 KB + stack |
| Rust + Embassy, `opt-level = "s"` (default) | **31.8 KB** (51%) | 2.4 KB + stack |

Roughly where the C flash goes: animations 4.0 KB, Startris 2.5 KB, LED
tables 2.2 KB, Snake 1.8 KB (its demo path search included), menu 1.5 KB,
Pong / Star Chase / game shell / name tag / Breakout about 1.1 to 1.3 KB
each, Flappy 0.7 KB. The 5x7 font is 295 bytes.

Rust is about 1.5 times the C size: about 3 KB of Embassy executor, timer
and HAL interrupt code, the rest is code generation. There is room left for
more games either way.

## What is solid and what is guessed

**Solid**

- ch32-hal has a `ch643` feature (Embassy time driver, RCC, GPIO, ADC, timers).
  The dependency is pinned to commit `4fde792`, because CH643 support is not on
  crates.io yet.
- The LED map is traced from the PCB nets.
- The TouchKey sequence follows WCH's own EVT example.

**Guessed** (each place is marked `GUESS` in the source)

- **LEDPWM.** The register offsets and field names come from WCH's `ch643.h`.
  WCH has published no reference manual and no LEDPWM example, so the bit
  meanings are inferred from the field names. The unknowns are:
  - the MATRIX_MODE value
  - how 24 PWM channels map onto the 16-bit `PWM_PIN` register
  - the DMA address and count format
  - the frame-done flag
  - the clock enable

  Ask WCH for the reference manual or the MounRiver LEDPWM library.
  If the hardware block cannot be made to work, write a software scanner
  (GPIO + timer). Only `ledpwm.rs` would change, but check the GPIO
  sink-current limits first: one COM pin can sink up to 24 LEDs at once.
- **Touch thresholds** (`PRESS_DELTA` / `RELEASE_DELTA`) are untuned. Log raw
  values on a real board and set them.

## Hardware notes found along the way

- **T2 may not work.** T2 is on ADC channel 11. The datasheet says channels 3, 7,
  11 and 15 are missing on chips whose lot number has 0 as the 5th digit from
  the end. Moving T2 to another TouchKey-capable pin avoids this.
- **Unbonded pads.** WCH's SystemInit pulls up the pads that the LQFP64
  package does not bond out (PC6–PC9, PC12, PC13, PC20); if they float they
  draw current. ch32-hal does not do this, so `main.rs` does it.
- **No battery gauge.** The board has no battery-voltage sense, so the
  firmware can show *that* the battery charges but not *how full* it is.
- **No USB driver yet.** ch32-hal has no USB driver for the CH643. The chip
  has USB FS device and USB PD, so this would need a port, for example from
  the CH32X035 USBFS driver.
