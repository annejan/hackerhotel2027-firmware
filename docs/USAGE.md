# Using the starfish badge

This guide covers the firmware in this repo (Rust in `src/`, C in `c/`).
Both versions behave the same. Nothing here has run on a real badge yet.
To try it on a PC, use the [emulator](#emulator).

## The buttons

The badge has four push buttons. Hold the badge by its two lower legs: your
thumbs are on the front buttons, your index fingers reach the two on the
back.

| Button | Where | Mostly used for |
|---|---|---|
| **◀** | front, lower left leg (SW3) | previous, left |
| **▶** | front, lower right leg (SW4) | next, right |
| **A** | back, right arm (SW2) | select; hold for the menu |
| **B** | back, left arm (SW1) | drop in Startris, previous position in EDIT NAME |

| Press | What it is |
|---|---|
| **Tap** | Press and let go within 0.6 s. |
| **Hold** | Keep it down for 0.6 s. The hold happens while you press; it does not wait for you to let go. |
| **Press** | The moment the button goes down. Games use this, so they react without waiting for you to let go. |
| **Chord** | Two or more buttons down at the same time. A chord does not count as a tap or a hold. |

So to hold a button, hold that button **alone**. If another button is also
down, it is a chord and nothing happens.

## Always available

| Do this | What happens |
|---|---|
| **Hold A** | The menu opens. This works from every screen, also in the middle of a game. |
| **Hold ◀ + ▶ for 3 s** | The badge reboots into the USB boot ROM for flashing. An arrow runs down the grid for a second first. See [Flashing](../README.md#flashing). |
| Plug in USB | A battery gauge shows while the battery charges. Tap any button to hide it. It does not cover a game that you are playing. |

## The menu

Hold A to open the menu. The item's name shows on the grid. Long names
scroll. The outline shows where you are in the list. The arm colour shows
the group:

| Arm colour | Group |
|---|---|
| Orange | Name tag, Hacker Hotel |
| Purple | Animations |
| Green | Games |
| White | Edit name, Light |

| In the menu | |
|---|---|
| ◀ / ▶ | Previous / next item |
| Tap A | Select the item, and close the menu |
| Hold A | Close the menu and keep what was running |
| No button for 20 s | The menu closes |

The items, in order:

1. **NAME**: your name tag. The badge starts on this screen.
2. **HACKER HOTEL**: "HH" over "2027", then "HACKER HOTEL 2027" scrolls
   by. The outline runs marquee lights like a hotel sign.
3. **RIPPLE**, **GALAXY**, **HEART**, **LIFE**, **FIRE**, **TWINKLE**:
   animations.
4. **SNAKE**, **STARTRIS**, **BREAKOUT**, **PONG**, **FLAPPY**,
   **STAR CHASE**: games.
5. **EDIT NAME**: type your name on the badge, see [Your name](#your-name).
6. **LIGHT**: brightness. Tap A to step through five levels: night,
   5%, 12.5%, 31% and full LED power. A bar on the grid shows the level.
   The menu stays open, so you can step again. The badge starts at 12.5%,
   because bare LEDs at close range are bright. Full power is for
   daylight, and it drains the battery fastest.

## Name tag, Hacker Hotel and the animations

| Button | |
|---|---|
| ◀ / ▶ | Previous / next screen. This skips EDIT NAME and LIGHT, and it stops at the games, because a press starts a game. |
| Tap A | Brightness, the same five levels as LIGHT |
| Hold A | Menu |

## Games

### Demo mode

Every game starts in **demo mode**: the game plays itself, and a dim
light runs slowly round the outline. **Press ◀, ▶ or B to start a real
game.** A does not start a game. In demo mode, tap A for brightness or
hold A for the menu.

At game over, the arms flash red and your score shows on the grid. Then
the game goes back to demo mode. If you win at Pong, the arms do not flash
red.

### Leaving a game

**Hold A** (alone) to open the menu, then pick something else. In a game,
a quick tap on A does something else: Startris uses it to rotate the
piece, and the other games ignore it.

### The games

| Game | Buttons | Score |
|---|---|---|
| **Snake** | ◀ turns left, ▶ turns right. You can press two turns ahead. | The food you eat, shown in green on the arms. From a score of 10, the snake moves faster. |
| **Startris** | ◀ / ▶ move the piece (hold to repeat). Tap A to rotate, B to drop. | The lines you clear. The arms also show the colour of the next piece. |
| **Breakout** | ◀ / ▶ move the paddle. Hold to keep it moving. | The bricks you break, in orange. Your lives (3) show in blue on the first arms. When all the bricks are gone, a faster wave starts. |
| **Pong** | You play against the badge. ◀ moves your paddle (left side) up, ▶ moves it down. Hold to keep it moving. | Your points show in blue on the left arms, the badge's in red on the right arms. First to 5 wins. |
| **Flappy** | Press ◀, ▶ or B to flap. | The pipes you pass, in gold. |
| **Star Chase** | A light runs round the outline. Press ◀, ▶ or B when it is beside the arm that shows green. | The grid shows your level. Each hit makes the light faster. After 3 misses, the game is over. |

The playing field is the octagon in the middle, and its edge is the wall.

## Your name

### Typing it on the badge

Open the menu (hold A), go to **EDIT NAME** and tap A. The name shows on
the grid. The character you are changing is bright in the middle, with a
blinking line under it. The outline shows how far into the name you are.

| Button | |
|---|---|
| ◀ / ▶ | Previous / next character. Hold to keep going. |
| Tap A | Next position. One blank after the end, it goes back to the first character. |
| Tap B | Previous position |
| **Hold A** | Save, and show the name tag |

The characters come in this order: blank, `A`–`Z`, `0`–`9`, then
`- . ! ? ' * : / # @`. From a blank, ▶ gives `A`. To make the name
shorter, set the characters at the end to blank. Names are up to 24
characters long.

If you press nothing for a minute, the editor closes and does not save.

The name is kept in the last 256-byte page of flash, so it is still there
after you switch off. A firmware update with `wchisp flash` keeps it;
`wchisp erase` clears it. Saving to flash is not tested on a badge yet.

### The default

Until you save a name, the badge shows the name the firmware was built
with:

```sh
BADGE_NAME="ANNE JAN" cargo build --release      # Rust
make -C c NAME="ANNE JAN"                        # C
```

The font has upper-case letters, digits and ` !'*-.:?/#@`. Lower case shows
as upper case. Names up to 3 characters stand still. Longer names scroll.

## Meeting other badges (infrared)

On the name tag, **tap B** to send your name over infrared. Point the top
of your badge, where the IR LED and receiver sit, at the other badge. A
white wave runs out along your arms while it sends (about a quarter of a
second).

The badge that receives it shows **HI**, then your name, with colours
running round its arms, and sends its own name back; your badge then
greets that one the same way. So one tap and both badges know who they
met. Tap any button to close a greeting early.

A greeting waits while the menu or EDIT NAME is open or a game is being
played, and shows when you are done. The answer goes out straight away.

Not tested on a badge yet. On the
current board the IR receiver has no supply (see the
[hardware notes](../README.md#hardware-notes-found-along-the-way)), so
receiving needs a board fix first.

## UART console

The UART header J3 (pin 1 RX, pin 2 TX, pin 4 GND; 115200 baud, 8N1)
prints a log: the name at power on, names saved, infrared packets sent
and received. Type a command and Enter:

| Command | |
|---|---|
| `help` | the commands and the list of items |
| `info` | firmware, name, what is showing, brightness |
| `name TEXT` | set the name, saved in flash like EDIT NAME |
| `show N` | show item N (`help` lists them; not EDIT NAME or LIGHT) |
| `level N` | brightness, 0 (night) to 4 (full) |
| `hello` | send the name over infrared, as B on the name tag does |
| `test` | the [bring-up test](#bring-up-test-first-boards) |
| `factory` | the [factory test](#factory-test-the-flashing-table); `factory skip` marks it as passed without running it |
| `sponsors` | play the [sponsor reel](#sponsor-reel) again |

Pin 3 of J3 is the badge's VCC (battery or USB), not
3.3 V: do not connect it to the adapter's supply.

## Factory test (the flashing table)

A freshly flashed badge starts with a short factory test, made for
volunteers at the flashing table: about 30 seconds per badge, pass or
fail, no serial terminal needed. It comes back at every power-on until it
has passed once. The badge keeps that in flash, in the same page as the
name. A later `wchisp flash` keeps that page, so an update does not bring
the test back; `wchisp erase` does. To run it again on purpose, type
`factory` on the UART console.

![factory test: LED, ROW, KEY, USB and IR, then OK](fw/factory.gif)

Each step shows its name for a second first. The outline fills up as the
steps go by.

| Step | The badge shows | You do |
|---|---|---|
| **LED** | Every LED on: white, then the arms red, green, blue, over and over | All lit, right colours: tap **B**. Something dark or the wrong colour: tap **A** |
| **ROW** | One row lit, moving top to bottom, then one column, moving left to right, then a few sets of dots on the outline and arms | Always exactly one row, column or set of dots, never two, none missing: tap **B**. Otherwise tap **A** |
| **KEY** | `<` `B` `A` `>` dimmed | Press each button once; each lights up. Goes on by itself |
| **USB** | `USB`, blinking until it sees USB power | Keep it plugged in. Goes on after a second of USB power |
| **IR** | Heard (top) and sent (bottom) test packets | Nothing. It passes on its own echo, or on test packets from another badge nearby |
| **OK** | Green arms, `OK` | Done. After 3 s the badge goes to its name tag |

A failed step blinks red arms and shows the step's name: put the badge
on the repair pile. Tap **B** to try that step once more. KEY fails after
20 s and USB and IR after 10 s; the UART log says what was missing
(`factory test: FAIL at KEY: missing: < . A .`).

On the current board revision the IR receiver has no supply (see the
hardware notes in the README), so the IR step fails there until that is
fixed.

Holding **A + B for 3 s** leaves the factory test for the full bring-up
test below. The badge then runs the factory test again at the next
power-on.

### Card for the flashing table

> 1. Flash the badge. Keep it on USB. It starts the test by itself.
> 2. **LED**: every LED lit? Arms go red, green, blue? Yes: **B**. No: **A**.
> 3. **ROW**: one line moving at a time, never two? Yes: **B**. No: **A**.
> 4. **KEY**: press ◀, B, A and ▶ once each.
> 5. **USB** and **IR** go by themselves. Keep it on USB.
> 6. Green **OK**: good badge. Red: repair pile.

## Sponsor reel

The first power-on after the factory test passed (in practice: when the
attendee first switches it on) plays a short sponsor reel once: "THANKS
TO", then each sponsor's logo scrolling by in greyscale, with the arms in
the sponsor's colours. Any tap skips it.
After that it never plays by itself again; `sponsors` on the UART console
plays it on demand.

![sponsor reel](fw/sponsors.gif)

The reel is a table at the top of `c/src/sponsors.c` and
`src/sponsors.rs`: each item is a text or a logo, and how the arms light
up. For now:

| Sponsor | On the grid | Arms |
|---|---|---|
| ALLNET | a glimpse of the globe (taller than the grid, panned diagonally as it scrolls), then the word | a blue wave |
| badge.team | the word mark | a rainbow sweeping diagonally across the star |
| Nicolai Electronics | icon and name | an orange and grey wave |
| deFEEST | the name, its outline letters filled | a yellow and blue wave |

The logos come from the SVGs in `assets/sponsors`. To add or change one,
put the SVG (or a PNG) there, add a line to the `LOGOS` table in
`tools/sim/logo.py` (how many grid rows it gets, an optional crop) and run:

```sh
python3 tools/sim/logo.py --preview=logos.png   # writes c/src/logos.h and src/logos.rs
```

A logo pixel's brightness is how much of it the picture covers, in four
steps; lone specks are dropped. A logo that fits the grid stands in the
middle for 2.5 s, fading in; a wider one scrolls by like text, and one
taller than the grid also pans from its top to its bottom. Word marks
work best at 9 to 16 rows; thin lines and small text get lost, so the
ALLNET globe is redrawn in the tool with fewer, thicker lines, and
deFEEST's outline letters are filled. A logo costs a quarter byte a
pixel: 100 to 250 bytes each.

## Bring-up test (first boards)

For checking a freshly built board: **hold A + B for 3 s**,
or type `test` on the UART console. Connect a serial terminal to J3 to
see what each step should look like; every step and every button event
is logged there. ◀ / ▶ go to the previous / next test, **hold A** leaves.

![bring-up test: every test in turn, from the firmware in tools/fwsim](fw/selftest.gif)

| Test | On the badge | In the log |
|---|---|---|
| COM | all 24 PWM channels on one COM line, stepping PB0..PB15 twice a second | `COM PB3 on` |
| PWM | one PWM channel on all COM lines, stepping PA0..PA23 | `PWM PA7 on` |
| LED | one LED at a time, 4 a second. B pauses; while paused, tap A steps | `slot 41: PB1 PA17 D35 green` |
| ALL | every LED on at the current brightness: measure the current here | the brightness level |
| GRY | grey ramp left to right on the grid; arms red, green, blue, white, orange | |
| KEY | the buttons held, as L, B, A, R | |
| SCN | LED rows per second | `scan: 2930 rows/s` means the LED clock is the 48 MHz HCLK |
| IR | sends a test packet every 2 s and counts how many it hears back (top: heard, bottom: sent) | `ir: heard 3 of 3 sent` |
| PWR | USB or BAT, and CHG while charging | `power: USB (PC3 high), charging (CHRG low)` |

The LED test names the designator of every slot (from the KiCad board),
so a dead LED or a swapped net shows up as "D35 should be on". The scan
rate checks the one timing guess in the LED driver. The IR test hears the
badge's own LED, so it tests sender and receiver together (it needs the
receiver's supply fixed, see the hardware notes).

## Setup page

Plug the badge into a computer and open the setup page,
[`docs/webusb/index.html`](webusb/index.html), in Chrome or Edge. Firefox
and Safari have no WebUSB. Click **Connect badge (USB)** and pick the
badge. Then you can:

- type your name and click **Save**. It is saved in flash, the same as
  EDIT NAME.
- pick what the badge shows (all screens and games, except the menu-only
  entries EDIT NAME and LIGHT).
- set the brightness.

The page reads the badge's state every second, so changes made with the
buttons show up on the page as well.

On Linux, give your user access to the badge's USB ID (1209:D035), for
example with this udev rule:
`SUBSYSTEM=="usb", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="d035", MODE="0666"`.

The USB part is not tested on a badge yet.

## Emulator

The emulator runs the real firmware code on a PC. The C or Rust firmware
is built as a shared library, and `tools/emu/emu.py` draws the LEDs live
on a picture of the board, with the four buttons where they are on the
badge (the two on the back drawn dashed):

```sh
make -C c emu && python3 tools/emu/emu.py c
(cd tools/fwsim && cargo build --release) && python3 tools/emu/emu.py rust
```

| Key | |
|---|---|
| `←` / `→` | ◀ / ▶ |
| `↑` or `A` | A |
| `↓` or `B` | B |
| click | You can also click the buttons on the board. |
| `C` | USB power on / off (charging) |
| `R` | Reset (power on again) |
| `Esc` | Quit |

The keys follow the same tap / hold / chord rules as the badge. Hold `↑`
and `↓` together for 3 s for the bring-up test. To open the
menu from a game, hold `↑` alone for 0.6 s. Hold `←` and `→`
together for 3 s to see the USB boot screen. The emulator then powers the
firmware on again. A name saved with EDIT NAME stays saved over `R` and the
reboot, as it would in flash. It is lost when you close the emulator.

Every emulator on the PC shares one infrared "room"
(UDP broadcast on 127.255.255.255:8028; `--no-ir` leaves it). Start two,
press `B` on one's name tag, and they swap names. Lines typed into the
terminal go to the badge's UART console (try `help`), and the badge's log
appears there too.

With `--fresh`, the emulator starts as a freshly flashed badge: the
factory test first (`C` is the USB power it waits for), and after `R` the
sponsor reel. Without it, the emulator is a badge that has been through
both.

With `--web`, the emulator also serves the setup page on
<http://localhost:8027/>. Click **Connect emulator** there: the page then
talks to the emulated badge the same way it talks to a real one over USB.

The emulator needs Python with tkinter, numpy, scipy and Pillow. The
interface between the emulator and the firmware is in
`tools/emu/badge_emu.h`.
