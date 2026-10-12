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

The USB part only exists in the C firmware so far, and it is not tested
on a badge yet.

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

The keys follow the same tap / hold / chord rules as the badge. To open the
menu from a game, hold `↑` alone for 0.6 s. Hold `←` and `→`
together for 3 s to see the USB boot screen. The emulator then powers the
firmware on again. A name saved with EDIT NAME stays saved over `R` and the
reboot, as it would in flash. It is lost when you close the emulator.

With `--web` (C firmware), the emulator also serves the setup page on
<http://localhost:8027/>. Click **Connect emulator** there: the page then
talks to the emulated badge the same way it talks to a real one over USB.

The emulator needs Python with tkinter, numpy, scipy and Pillow. The
interface between the emulator and the firmware is in
`tools/emu/badge_emu.h`.
