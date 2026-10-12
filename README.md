# hh27-starfish firmware (theoretical)

Embassy firmware for the Hacker Hotel 2027 starfish badge. It compiles, but it
has never run on a board.

## Hardware

| | |
|---|---|
| MCU | WCH **CH643Q**: QingKe V4C RISC-V (RV32IMAC), 48 MHz, 62 KiB flash, 20 KiB RAM, LQFP64 |
| LEDs | 384 slots in a 24 × 16 matrix: PA0–PA23 drive the anodes (PWM), PB0–PB15 sink the cathodes (COM) |
| | 208 mono LEDs in a 17 × 16 grid, 71 mono LEDs on the outline, 35 RGB LEDs (7 per arm) |
| Touch | T1–T4 on PC0–PC3, TouchKey channels 10–13 |
| Power | LiPo with XT4054 charger; CHRG on PB22 |
| Debug | SWIO / SWCK (WCH-LinkE) |

## Build

```sh
cargo build --release            # stable Rust, target riscv32imc-unknown-none-elf
cargo run --release              # flashes with wlink (WCH-LinkE)
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

## Structure

| File | What |
|---|---|
| `main.rs` | Embassy main: render loop at 30 fps, event channel, CHRG monitor task |
| `touch.rs` | TouchKey polling task: baseline tracking, tap and hold events |
| `anim.rs` | Animations ported from `tools/sim/hh2027.py`: ripple, galaxy, heartbeat, scroller, life, fire, twinkle, charging |
| `frame.rs` | Frame buffer, integer HSV, sin table helpers, RNG |
| `ledpwm.rs` | CH643 LEDPWM driver: **mostly guesswork, see below** |
| `generated.rs` | LED map, polar coordinates, gamma and sine tables (generated) |

`tools/sim/hh2027.py` renders animation previews (GIF + MP4) on a KiCad 3D
render of the real board, with the real LED positions. The firmware
animations are ports of these. Run `python3 tools/sim/hh2027.py [ripple fire ...]`;
the output goes to `previews/`. It needs numpy, scipy, Pillow and ffmpeg, and
`--refresh` re-reads the board (kicad-cli + pcbnew).

Controls: T1 next, T2 previous, T3 brightness, T4 restart. Hold any pad to
toggle night mode. While charging, a battery gauge shows until the next tap.

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
- **No battery gauge.** The board has no battery-voltage sense, so the
  firmware can show *that* the battery charges but not *how full* it is.
- **No USB driver yet.** ch32-hal has no USB driver for the CH643. The chip
  has USB FS device and USB PD, so this would need a port, for example from
  the CH32X035 USBFS driver.
