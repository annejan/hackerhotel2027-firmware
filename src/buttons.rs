//! The four push buttons. Each has a 10k pull-up to VCC on the board and
//! pulls its pin low while pressed.
//!
//!   index  name  switch  where                                 pin
//!   0      ◀     SW3     front, lower left leg (left thumb)    PC5
//!   1      B     SW1     back, left arm (left finger)          PB20
//!   2      A     SW2     back, right arm (right finger)        PC4
//!   3      ▶     SW4     front, lower right leg (right thumb)  PB18
//!
//! The index is what the rest of the firmware sees (`Event::Press(i)`,
//! bit i of `held()`): ◀ / ▶ step and steer, A selects (hold: menu), B goes
//! back or drops.

use embassy_time::{Duration, Ticker};
use hal::gpio::Input;

use ch32_hal as hal;

use core::sync::atomic::{AtomicU8, Ordering};

use crate::{Event, EVENTS};

/// Buttons down right now, bit n = button index n (load/store only: no CAS
/// on this core without the A extension in the target).
static HELD: AtomicU8 = AtomicU8::new(0);

pub fn held() -> u8 {
    HELD.load(Ordering::Relaxed)
}

pub const BUTTONS: usize = 4;
const POLL_MS: u64 = 10;
/// A change has to be seen on this many polls in a row (contact bounce).
const DEBOUNCE: u8 = 2;
pub const HOLD_MS: u64 = 600;
const HOLD_TICKS: u16 = (HOLD_MS / POLL_MS) as u16;
/// Hold these two (◀ + ▶) together this long to reboot into the USB boot ROM.
pub const BOOT_CHORD: (usize, usize) = (0, 3);
pub const BOOT_MS: u64 = 3000;
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

/// `pins` in index order: ◀ (PC5), B (PB20), A (PC4), ▶ (PB18), as floating
/// inputs (the pull-ups are on the board).
#[embassy_executor::task]
pub async fn scan(pins: [Input<'static>; BUTTONS]) {
    let mut buttons: [Button; BUTTONS] = Default::default();
    let mut ticker = Ticker::every(Duration::from_millis(POLL_MS));
    loop {
        for (i, (b, pin)) in buttons.iter_mut().zip(pins.iter()).enumerate() {
            b.released = false;
            if b.down {
                b.ticks = b.ticks.saturating_add(1);
            }
            if pin.is_low() == b.down {
                b.bounce = 0;
            } else {
                b.bounce += 1;
                if b.bounce >= DEBOUNCE {
                    b.bounce = 0;
                    b.down = !b.down;
                    if b.down {
                        b.ticks = 0;
                        let _ = EVENTS.try_send(Event::Press(i as u8));
                    } else {
                        b.released = true;
                    }
                }
            }
        }

        HELD.store(buttons.iter().enumerate().fold(0, |m, (i, b)| m | (b.down as u8) << i), Ordering::Relaxed);

        if buttons.iter().filter(|b| b.down).count() > 1 {
            buttons.iter_mut().filter(|b| b.down).for_each(|b| b.chord = true);
        }
        for (i, b) in buttons.iter_mut().enumerate() {
            if b.released {
                if !b.chord && b.ticks < HOLD_TICKS {
                    let _ = EVENTS.try_send(Event::Tap(i as u8));
                }
                b.chord = false;
            } else if b.down && !b.chord && b.ticks == HOLD_TICKS {
                let _ = EVENTS.try_send(Event::Hold(i as u8));
            }
        }

        let (a, b) = (&buttons[BOOT_CHORD.0], &buttons[BOOT_CHORD.1]);
        if a.down && b.down && a.ticks.min(b.ticks) == BOOT_TICKS {
            let _ = EVENTS.try_send(Event::Bootloader);
        }

        ticker.next().await;
    }
}
