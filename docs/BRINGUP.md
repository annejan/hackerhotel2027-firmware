# Bring-up: the first prototype

Neither firmware has run on a board yet. Everything in it was checked
against the CH643 manuals (reference manual V1.9, datasheet V1.4, QingKe
V4 manual) and tested on the PC, but a few things only a real board can
settle. This is the order to check them in on the first prototype, so a
problem shows up in the step that can explain it, before a later step
trips over it.

You need a bench supply with a current limit, a scope, a USB-UART adapter
for J3 (3.3 V logic, **do not** connect its supply pin: pin 3 of J3 and
J4 is the badge's VCC, up to ~5 V), a WCH-LinkE for J4, and
[wchisp](https://github.com/ch32-rs/wchisp). Flash either firmware; the C
one is smaller and logs the same.

Memory reads and writes below are given as OpenOCD commands (`mdw ADDR
[COUNT]`, `mww ADDR VALUE`), as in WCH's OpenOCD from MounRiver with the
WCH-LinkE. Any SWD tool that can read and write words does.

## 1. Before power-on: the LED current

The column resistors R1..R24 set the LED current (the firmware leaves the
CH643's built-in constant current off). With all 384 slots populated, one
COM line can carry 24 columns at once, so the peak current is 24 × the
column current, at every brightness. The datasheet allows **550 mA in
total** through the VDD pins (table 3-1). At 20 mA per column a white row
is 480 mA: too close. Aim for 5-10 mA per column.

First power-up: bench supply on the battery connector, 3.7 V, limit
about 300 mA, no USB.

## 2. SWD: what chip is this

On J4 (1 SWIO, 2 SWCK, 3 VCC, 4 GND):

| Read | What | Expect |
|---|---|---|
| `mdw 0x1FFFF704` | chip ID | bits 7:4 are the silicon revision; 0 means early silicon, where GPIO CFGHR cannot be read back (neither firmware reads it back) |
| `mdw 0x1FFFF7E0` | flash size in KiB (low half) | the CH643 has 62 KiB for code |
| `mdw 0x1FFFF7E8 3` | the unique ID | the badge's USB serial number is made from it |

Dump the option bytes and FLASH_WPR too: the last two pages (0x0800F600
and 0x0800F700) must not be write-protected, and read protection must be
off.

## 3. Clock and UART

Flash the firmware (hold ◀ + ▶ while plugging in USB, then `wchisp
flash`, or over SWD). Open J3 at 115200 8N1. Expect at power-on:

```
hh27 starfish badge, c firmware. Name: ...
Type help.
```

A readable banner proves the clock (HSI at 48 MHz, the flash wait
states), the USART3 remap to PB16/PB17 and the pin setup in one go.
Garbage at 115200 means the clock is wrong. Measure one bit on the scope
(8.68 µs): the HSI is specified at ±2 %, and USB needs much better than
that (step 9).

## 4. LED matrix

Start at the lowest brightness (`level 0` on the console).

- **Scan rate.** Scope a COM line (PB0, PB1): active-low pulses at
  about 183 Hz, one row about 341 µs plus ~5 µs of blanking. Or run the
  bring-up test (hold A + B 3 s, or `test` on the console) to SCN, which
  logs `scan: 2930 rows/s`. That proves the LED block runs on the 48 MHz
  HCLK. 0 rows/s means the scanner stopped: the firmware stops waiting
  for it and keeps running, but the grid stays dark.
- **Rows show garbage, or the grid stays dark with a normal scan rate.**
  The DMA address register is 16 bits wide; the firmware writes the low
  half of the RAM address of the row buffer. The manual does not say how
  the block tells RAM from flash. Next suspect.
- **Intensity.** Step `level 0` to `level 4`: the badge should get
  brighter in even steps. The firmware assumes the low 6 bits of each
  intensity byte count. If the steps look wrong, the hardware may use
  the top 6 bits instead.
- **Current.** In the bring-up test's ALL step (every LED on at the
  current brightness), measure the supply current, the voltage on a COM
  pin and how warm U1 gets. Do this before the factory test's white
  step, which lights everything at full brightness.
- **Every LED.** The bring-up test's LED step lights one LED at a time
  and logs its designator (`slot 41: PB1 PA17 D35 green`), so a dead LED
  or swapped net names itself.

## 5. Inputs and power

- Buttons: ◀ PC4, ▶ PC5, A PB18, B PB20. The bring-up test's KEY step
  shows each press.
- CHRG (PB22) and VBUS (PC3): the PWR step logs `power: USB (PC3
  high), charging (CHRG low)`.
- With USB in and the grid at full brightness, measure VBUS − VCC (the
  drop over D315). PC3 sees VBUS directly while the chip runs on VCC,
  and the datasheet allows only VDD + 0.3 V on a pin; this measurement
  sizes the series resistor proposed for PC3.

## 6. Flash store

Set a name (`name TEST` on the console), power off and on: the banner
shows it again. Then dump both pages:

```
mdw 0x0800F600 64
mdw 0x0800F700 64
```

The pages take turns, so each save goes to the other one. Each starts
with `0x4E414D45` ("NAME") and the name length; word 60 counts the saves,
word 61 is a check over the page, word 62 the challenges' progress
(`0x5052....`), word 63 the stage (`TEST` / `DONE` in ASCII). If a save
fails, the log says `name NOT saved (flash write failed)`.

While a page is written (~5 ms) the CPU waits for the flash, so the
grid may flicker once. The Rust firmware also turns interrupts off for
that time (the C one does not); both should be fine.

## 7. IR

Scope PB21 during the bring-up test's IR step (or `hello` on the
console): bursts of 38 kHz, low (LED on) a third of each period, high
when idle. PA12 must not change: it is an LED column, and also where the
IR timer's complementary output would go if it were enabled.

The IR step counts the packets the badge hears from itself (`ir: heard
3 of 3 sent`). After that, two badges: B on the name tag sends your name,
the other badge greets you and answers. Then the factory test's IR step.

## 8. SysTick compare

The C firmware times IR sends with the SysTick compare. The QingKe manual
does not say whether the compare fires when the count *passes* the
compare value, or only when it is equal. The firmware copes with both
now, but it is good to know. Do this right after a reset, before
anything has sent IR (the firmware only turns the SysTick interrupt on
for a send), with the firmware stopped in the debugger:

```
mww 0xE000F004 0          # clear the flag
mdw 0xE000F008 2          # the count: low, high word
mww 0xE000F014 <high>     # compare high = count high
mww 0xE000F010 <low - 1000>
# resume briefly, stop again
mdw 0xE000F004            # 1: fires on >=; 0: only on ==
```

## 9. USB

Both firmwares always use the CH643's 5 V USB setup (the PHY behind its
own 3.3 V regulator, 10k pull-up on D+), because USB is only there when
VDD comes from VBUS through the diode, about 4.7 V.

- Boot with USB in, and boot on the battery and plug USB in later. In
  both cases D+ idles at about 3.3 V, not at VDD.
- Enumerate on Linux, Windows and macOS. Then open the setup page
  (`docs/webusb/index.html`) and set a name, also while a game runs.
- USB runs from the internal oscillator. If enumeration is flaky, the
  HSI is the first suspect (step 3); the oscillator can be trimmed.
- USB is polled from the main loop. A setup packet that arrives while a
  flash page is written (~5 ms) gets no answer; the host retries it.
  Watch for failed requests while saving a name from the setup page.

## 10. Boot ROM

Hold ◀ + ▶ while plugging in USB, then `wchisp info`. Check whether the
boot ROM stays in ISP mode when nothing talks to it, or starts the
firmware after a while, and that a reset afterwards starts the firmware.
Holding ◀ + ▶ for 3 s while running also gets there (`bootloader.c`).

## 11. Soak

Leave it running on USB for more than half an hour (the SysTick low word
wraps every 716 s; it should wrap twice) with IR active, for example two
badges and `hello` now and then. Nothing may freeze: not the grid, not
IR, not the console.

## Then

- Run the factory test as the volunteers will (the card is in
  [USAGE.md](USAGE.md#card-for-the-flashing-table)).
- Pull the hardware repository and regenerate the LED tables if anything
  moved (`tools/gen_layout.py`).
