//! Name entry on the badge (EDIT NAME in the menu).
//!
//!   ◀ / ▶   previous / next character (hold to keep going)
//!   tap A    next position; past the end it wraps to the start
//!   tap B    previous position
//!   hold A   save and show the name tag
//! Left alone for a minute, it closes without saving.
//!
//! The grid shows the name with the character being edited in the middle,
//! bright, with a blinking underline; the outline shows the position.

use crate::anim::FPS;
use crate::frame::{draw_text, Frame, Rgb};
use crate::game::{BTN_LEFT, BTN_RIGHT};
use crate::generated::{EDGE, GRID_W, RGB};
use crate::nametag::{Name, MAX};
use crate::Event;

/// The characters ◀ / ▶ step through, in this order; from a blank, ▶
/// gives A.
const CHARS: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-.!?'*:/#@";
const TIMEOUT: u32 = 60 * FPS;
const REPEAT_AFTER: u8 = 12; // frames before a held button repeats
const REPEAT_EVERY: u8 = 4;

pub enum Done {
    Save(Name),
    Cancel,
}

pub struct Editor {
    buf: [u8; MAX],
    cursor: usize,
    rep: [u8; 2], // frames ◀ / ▶ held
    idle: u32,
    t: u32,
}

impl Editor {
    pub fn new(name: &Name) -> Self {
        let mut buf = [b' '; MAX];
        buf[..name.bytes().len()].copy_from_slice(name.bytes());
        Self { buf, cursor: 0, rep: [0; 2], idle: 0, t: 0 }
    }

    fn step(&mut self, d: isize) {
        let i = CHARS.iter().position(|&c| c == self.buf[self.cursor]).unwrap_or(0) as isize;
        self.buf[self.cursor] = CHARS[(i + d).rem_euclid(CHARS.len() as isize) as usize];
    }

    /// One past the last character that is not a space.
    fn end(&self) -> usize {
        self.buf.iter().rposition(|&c| c != b' ').map_or(0, |i| i + 1)
    }

    pub fn handle(&mut self, ev: Event) -> Option<Done> {
        self.idle = 0;
        match ev {
            Event::Press(0) => {
                self.step(-1);
                self.rep[0] = 0;
            }
            Event::Press(3) => {
                self.step(1);
                self.rep[1] = 0;
            }
            Event::Tap(2) => {
                // one further, up to one blank after the name, then back to the start
                self.cursor = if self.cursor + 1 > self.end().min(MAX - 1) { 0 } else { self.cursor + 1 };
            }
            Event::Tap(1) => self.cursor = self.cursor.saturating_sub(1),
            Event::Hold(2) => return Some(Done::Save(Name::new(&self.buf))),
            _ => {}
        }
        None
    }

    pub fn render(&mut self, f: &mut Frame, held: u8) -> Option<Done> {
        self.t = self.t.wrapping_add(1);
        self.idle += 1;
        if self.idle > TIMEOUT {
            return Some(Done::Cancel);
        }
        for (k, btn, d) in [(0, BTN_LEFT, -1), (1, BTN_RIGHT, 1)] {
            if held & btn != 0 {
                self.rep[k] = self.rep[k].saturating_add(1);
                if self.rep[k] > REPEAT_AFTER && self.rep[k] % REPEAT_EVERY == 0 {
                    self.step(d);
                }
            } else {
                self.rep[k] = 0;
            }
        }

        // the name, with the cursor's character at columns 6..=10
        let c0 = (GRID_W as i32 - 5) / 2;
        for (i, &c) in self.buf.iter().enumerate() {
            let x = c0 + (i as i32 - self.cursor as i32) * 6;
            if x > -6 && x < GRID_W as i32 {
                let ch = [c];
                let s = core::str::from_utf8(&ch).unwrap_or(" ");
                draw_text(f, s, x, 3, if i == self.cursor { 255 } else { 60 });
            }
        }
        let blink = if self.t % 20 < 12 { 255 } else { 40 };
        for c in 0..5 {
            f.grid(11, c0 as usize + c, blink);
        }
        // the outline marks the position in the name
        let (a, b) = (self.cursor * EDGE.len() / MAX, (self.cursor + 1) * EDGE.len() / MAX);
        for (j, &s) in EDGE.iter().enumerate() {
            f.set(s, if (a..b).contains(&j) { 220 } else { 12 });
        }
        for i in 0..RGB.len() {
            f.rgb(i, Rgb(40, 200, 255).scaled(70));
        }
        None
    }
}
