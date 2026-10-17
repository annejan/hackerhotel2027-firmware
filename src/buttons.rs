//! The four push buttons. Each has a 10k pull-up to VCC on the board and
//! pulls its pin low while pressed.
//!
//! All four are on the back; left and right as seen from the front.
//!
//!   index  name  switch  where             pin
//!   0      ◀     SW3     lower left leg    PC4
//!   1      B     SW1     left arm          PB20
//!   2      A     SW2     right arm         PC5
//!   3      ▶     SW4     lower right leg   PB18
//!
//! The index is what the rest of the firmware sees (`Event::Press(i)`,
//! bit i of `held()`): ◀ / ▶ step and steer, A selects (hold: menu), B goes
//! back or drops.

use crate::pac::{GPIOB, GPIOC};
use crate::Event;

pub const BUTTONS: usize = 4;
pub const POLL_MS: u32 = 10;
/// A change has to be seen on this many polls in a row (contact bounce).
const DEBOUNCE: u8 = 2;
pub const HOLD_MS: u32 = 600;
const HOLD_TICKS: u16 = (HOLD_MS / POLL_MS) as u16;
/// Hold these two (◀ + ▶) together this long to reboot into the USB boot ROM.
pub const BOOT_CHORD: (usize, usize) = (0, 3);
/// And these two (A + B) for the bring-up test.
pub const TEST_CHORD: (usize, usize) = (2, 1);
pub const BOOT_MS: u32 = 3000;
const BOOT_TICKS: u16 = (BOOT_MS / POLL_MS) as u16;

#[derive(Default)]
struct Button {
    down: bool,
    /// Polls in a row that disagreed with `down`.
    bounce: u8,
    /// Released during the last poll.
    released: bool,
    /// Was down together with another button: part of a chord, so it sends
    /// no Tap or Hold of its own.
    chord: bool,
    ticks: u16,
}

/// Pin of each button, in index order: ◀ PC4, B PB20, A PC5, ▶ PB18.
/// (port, pin): false = GPIOC, true = GPIOB.
const PINS: [(bool, u32); BUTTONS] = [(false, 4), (true, 20), (false, 5), (true, 18)];

pub struct Buttons {
    buttons: [Button; BUTTONS],
    /// Buttons down right now, bit n = button index n.
    held: u8,
}

impl Buttons {
    /// The four pins as floating inputs (0b0100; the pull-ups are on the
    /// board). Their ports' clocks are on already (main::clock_init).
    pub fn new() -> Self {
        GPIOC.cfglr().modify(|w| w.0 = (w.0 & !(0xFF << 16)) | 0x44 << 16); // PC4, PC5
        GPIOB.cfgxr().modify(|w| w.0 = (w.0 & !(0xF << 16 | 0xF << 8)) | 0x4 << 16 | 0x4 << 8); // PB20, PB18
        Self { buttons: Default::default(), held: 0 }
    }

    pub fn held(&self) -> u8 {
        self.held
    }

    /// Call every `POLL_MS`; sends Press/Tap/Hold/Bootloader/Selftest.
    pub fn poll(&mut self, mut send: impl FnMut(Event)) {
        let (b_in, c_in) = (GPIOB.indr().read().0, GPIOC.indr().read().0);
        let buttons = &mut self.buttons;
        for (i, (b, &(port_b, pin))) in buttons.iter_mut().zip(PINS.iter()).enumerate() {
            let low = (if port_b { b_in } else { c_in }) & 1 << pin == 0;
            b.released = false;
            if b.down {
                b.ticks = b.ticks.saturating_add(1);
            }
            if low == b.down {
                b.bounce = 0;
            } else {
                b.bounce += 1;
                if b.bounce >= DEBOUNCE {
                    b.bounce = 0;
                    b.down = !b.down;
                    if b.down {
                        b.ticks = 0;
                        send(Event::Press(i as u8));
                    } else {
                        b.released = true;
                    }
                }
            }
        }

        self.held = buttons.iter().enumerate().fold(0, |m, (i, b)| m | (b.down as u8) << i);

        if buttons.iter().filter(|b| b.down).count() > 1 {
            buttons.iter_mut().filter(|b| b.down).for_each(|b| b.chord = true);
        }
        for (i, b) in buttons.iter_mut().enumerate() {
            if b.released {
                if !b.chord && b.ticks < HOLD_TICKS {
                    send(Event::Tap(i as u8));
                }
                b.chord = false;
            } else if b.down && !b.chord && b.ticks == HOLD_TICKS {
                send(Event::Hold(i as u8));
            }
        }

        for (chord, ev) in [(BOOT_CHORD, Event::Bootloader), (TEST_CHORD, Event::Selftest)] {
            let (a, b) = (&buttons[chord.0], &buttons[chord.1]);
            if a.down && b.down && a.ticks.min(b.ticks) == BOOT_TICKS {
                send(ev);
            }
        }
    }
}
