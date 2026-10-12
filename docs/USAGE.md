# Using the starfish badge

This guide covers the firmware in this repo (Rust in `src/`, C in `c/`).
Both versions behave the same. Nothing here has run on a real badge yet.
To try it on a PC, use the [emulator](#emulator).

## The touch pads

The badge has four touch pads, **T1 to T4**. Every control is a touch on
one of them:

| Touch | What it is |
|---|---|
| **Tap** | Touch and let go within 0.6 s. |
| **Hold** | Keep touching for 0.6 s. The hold happens while you touch; it does not wait for you to let go. |
| **Press** | The moment you touch. Games use this, so they react without waiting for you to let go. |
| **Chord** | Two or more pads touched at the same time. A chord does not count as a tap or a hold. |

So to hold a pad, hold that pad **alone**. If another pad is also down, it
is a chord and nothing happens.

## Always available

| Do this | What happens |
|---|---|
| **Hold T3** | The menu opens. This works from every screen, also in the middle of a game. |
| **Hold T1 + T4 for 3 s** | The badge reboots into the USB boot ROM for flashing. An arrow runs down the grid for a second first. See [Flashing](../README.md#flashing). |
| Plug in USB | A battery gauge shows while the battery charges. Tap any pad to hide it. It does not cover a game that you are playing. |

## The menu

Hold T3 to open the menu. The item's name shows on the grid. Long names
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
| T1 / T4 | Previous / next item |
| Tap T3 | Select the item, and close the menu |
| Hold T3 | Close the menu and keep what was running |
| No touch for 20 s | The menu closes |

The items, in order:

1. **NAME**: your name tag. The badge starts on this screen.
2. **HACKER HOTEL**: "HH" over "2027", then "HACKER HOTEL 2027" scrolls
   by. The outline runs marquee lights like a hotel sign.
3. **RIPPLE**, **GALAXY**, **HEART**, **LIFE**, **FIRE**, **TWINKLE**:
   animations.
4. **SNAKE**, **STARTRIS**, **BREAKOUT**, **PONG**, **FLAPPY**,
   **STAR CHASE**: games.
5. **EDIT NAME**: type your name on the badge, see [Your name](#your-name).
6. **LIGHT**: brightness. Tap T3 to step through five levels: night,
   5%, 12.5%, 31% and full LED power. A bar on the grid shows the level.
   The menu stays open, so you can step again. The badge starts at 12.5%,
   because bare LEDs at close range are bright. Full power is for
   daylight, and it drains the battery fastest.

## Name tag, Hacker Hotel and the animations

| Touch | |
|---|---|
| T1 / T4 | Previous / next screen. This skips EDIT NAME and LIGHT, and it stops at the games, because a touch starts a game. |
| Tap T3 | Brightness, the same five levels as LIGHT |
| Hold T3 | Menu |

## Games

### Demo mode

Every game starts in **demo mode**: the game plays itself, and a dim
light runs slowly round the outline. **Touch T1, T2 or T4 to start a real
game.** T3 does not start a game. In demo mode, tap T3 for brightness or
hold T3 for the menu.

At game over, the arms flash red and your score shows on the grid. Then
the game goes back to demo mode. If you win at Pong, the arms do not flash
red.

### Leaving a game

**Hold T3** (alone) to open the menu, then pick something else. In a game,
a quick tap on T3 does something else: Startris uses it to rotate the
piece, and the other games ignore it.

### The games

| Game | Pads | Score |
|---|---|---|
| **Snake** | T1 turns left, T4 turns right. You can press two turns ahead. | The food you eat, shown in green on the arms. From a score of 10, the snake moves faster. |
| **Startris** | T1 / T4 move the piece (hold to repeat). Tap T3 to rotate, T2 to drop. | The lines you clear. The arms also show the colour of the next piece. |
| **Breakout** | T1 / T4 move the paddle. Hold to keep it moving. | The bricks you break, in orange. Your lives (3) show in blue on the first arms. When all the bricks are gone, a faster wave starts. |
| **Pong** | You play against the badge. T1 moves your paddle (left side) up, T4 moves it down. Hold to keep it moving. | Your points show in blue on the left arms, the badge's in red on the right arms. First to 5 wins. |
| **Flappy** | Touch T1, T2 or T4 to flap. | The pipes you pass, in gold. |
| **Star Chase** | A light runs round the outline. Touch T1, T2 or T4 when it is beside the arm that shows green. | The grid shows your level. Each hit makes the light faster. After 3 misses, the game is over. |

The playing field is the octagon in the middle, and its edge is the wall.

**T2 may not work** on some badges (see the
[hardware notes](../README.md#hardware-notes-found-along-the-way)). Only
Startris needs T2, and only to drop the piece. Every game works without
it.

## Your name

### Typing it on the badge

Open the menu (hold T3), go to **EDIT NAME** and tap T3. The name shows on
the grid. The character you are changing is bright in the middle, with a
blinking line under it. The outline shows how far into the name you are.

| Touch | |
|---|---|
| T1 / T4 | Previous / next character. Hold to keep going. |
| Tap T3 | Next position. One blank after the end, it goes back to the first character. |
| Tap T2 | Previous position |
| **Hold T3** | Save, and show the name tag |

The characters come in this order: blank, `A`–`Z`, `0`–`9`, then
`- . ! ? ' * : / # @`. From a blank, T4 gives `A`. To make the name
shorter, set the characters at the end to blank. Names are up to 24
characters long.

If you touch nothing for a minute, the editor closes and does not save.

Tapping T3 is the only way to move along the name without T2: past the end
it goes back to the first character. So the editor works on badges where T2
does not.

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
pads show up on the page as well.

On Linux, give your user access to the badge's USB ID (1209:D035), for
example with this udev rule:
`SUBSYSTEM=="usb", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="d035", MODE="0666"`.

The USB part only exists in the C firmware so far, and it is not tested
on a badge yet.

## Emulator

The emulator runs the real firmware code on a PC. The C or Rust firmware
is built as a shared library, and `tools/emu/emu.py` draws the LEDs live
on a picture of the board:

```sh
make -C c emu && python3 tools/emu/emu.py c
(cd tools/fwsim && cargo build --release) && python3 tools/emu/emu.py rust
```

| Key | |
|---|---|
| `1` `2` `3` `4` or `←` `↓` `↑` `→` | Pads T1, T2, T3, T4. You can also click the pads under the board. |
| `C` | USB power on / off (charging) |
| `R` | Reset (power on again) |
| `Esc` | Quit |

The keys follow the same tap / hold / chord rules as the badge. To open the
menu from a game, hold `3` (or `↑`) alone for 0.6 s. Hold `1` and `4`
together for 3 s to see the USB boot screen. The emulator then powers the
firmware on again. A name saved with EDIT NAME stays saved over `R` and the
reboot, as it would in flash. It is lost when you close the emulator.

With `--web` (C firmware), the emulator also serves the setup page on
<http://localhost:8027/>. Click **Connect emulator** there: the page then
talks to the emulated badge the same way it talks to a real one over USB.

The emulator needs Python with tkinter, numpy, scipy and Pillow. The
interface between the emulator and the firmware is in
`tools/emu/badge_emu.h`.
