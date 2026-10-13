//! Higher or lower; same as `c/src/games/hilo.c`: a number from 1 to 99;
//! will the next one be higher (▶) or lower (◀)? A right guess adds one to
//! the streak, which fills the outline; a wrong one ends the game.
//! Challenge 3 (challenge.rs).

use crate::font::text_width;
use crate::frame::{draw_text, Frame, Rgb, Rng};
use crate::game::{arm_line, Input, Play, ARM_LINE, BTN_LEFT, BTN_RIGHT, BUTTON_ARM};
use crate::generated::{EDGE, GRID_W, RGB};

const SHOW_FRAMES: u16 = 24;
const DEMO_THINK: u16 = 20;

pub struct Hilo {
    now: u8,
    next: u8,
    showing: bool, // the next number is out
    right: bool,   // the last guess
    streak: u16,
    t: u16,
}

fn pick(rng: &mut Rng, not: u8) -> u8 {
    loop {
        let n = (1 + rng.next() % 99) as u8;
        if n != not {
            return n;
        }
    }
}

impl Play for Hilo {
    const USES_EDGE: bool = true;

    fn new(rng: &mut Rng) -> Self {
        let now = pick(rng, 0);
        Self { now, next: pick(rng, now), showing: false, right: false, streak: 0, t: 0 }
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        self.t += 1;
        if self.showing {
            if self.t < SHOW_FRAMES {
                return true;
            }
            if !self.right {
                return false;
            }
            self.now = self.next;
            self.next = pick(rng, self.now);
            self.showing = false;
            self.t = 0;
            return true;
        }
        let guess = if demo {
            if self.t >= DEMO_THINK { Some(self.now < 50) } else { None }
        } else if inp.pressed & BTN_RIGHT != 0 {
            Some(true)
        } else if inp.pressed & BTN_LEFT != 0 {
            Some(false)
        } else {
            None
        };
        let Some(higher) = guess else { return true };
        self.right = (self.next > self.now) == higher;
        if self.right {
            self.streak += 1;
        }
        self.showing = true;
        self.t = 0;
        true
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        let n = if self.showing { self.next } else { self.now };
        let digits = [b'0' + n / 10, b'0' + n % 10];
        let s = core::str::from_utf8(if n >= 10 { &digits } else { &digits[1..] }).unwrap_or("");
        draw_text(f, s, (GRID_W - text_width(s)) as i32 / 2, 4, 255);
        // the streak, 5 outline LEDs a point
        for (j, &e) in EDGE.iter().enumerate() {
            f.set(e, if j < self.streak as usize * 5 { 120 } else { 0 });
        }
        if self.showing {
            let c = if self.right { Rgb(0, 255, 60) } else { Rgb(255, 20, 0) };
            for i in 0..RGB.len() {
                f.rgb(i, c);
            }
        } else {
            // ▶'s leg says higher, ◀'s lower
            let blink = if t % 30 < 20 { 160 } else { 60 };
            for k in 0..ARM_LINE {
                arm_line(f, BUTTON_ARM[3], k, Rgb(0, 200, 80).scaled(blink));
                arm_line(f, BUTTON_ARM[0], k, Rgb(255, 60, 0).scaled(blink));
            }
        }
    }

    fn score(&self) -> u16 {
        self.streak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_guesser_reaches_level_5() {
        let mut rng = Rng::new(7);
        let mut g = Hilo::new(&mut rng);
        while g.streak < 12 {
            let press = if g.next > g.now { BTN_RIGHT } else { BTN_LEFT };
            let inp = if g.showing { Input::default() } else { Input { pressed: press, held: 0 } };
            assert!(g.tick(false, inp, &mut rng), "right guesses never end it");
        }
        // a wrong one does
        while g.showing {
            g.tick(false, Input::default(), &mut rng);
        }
        while !g.showing {
            let wrong = if g.next > g.now { BTN_LEFT } else { BTN_RIGHT };
            g.tick(false, Input { pressed: wrong, held: 0 }, &mut rng);
        }
        assert!((0..SHOW_FRAMES).any(|_| !g.tick(false, Input::default(), &mut rng)));
    }
}
