//! Whack-a-mole; same as `c/src/games/whack.c`: a random arm lights up
//! from the tip towards the centre, one LED of its line at a time. Stop it
//! with any button before it reaches the centre: the sooner, the more
//! points (5 with one LED lit, 2 with four). Reaching the centre ends the
//! game, and so does pressing while nothing is lit. It speeds up as you
//! go. The grid shows the points. Challenge 2 (challenge.rs).

use crate::anim::FPS;
use crate::frame::{draw_number, Frame, Rgb, Rng};
use crate::game::{arm_line, arm_sides, Input, Play, ARM_LINE};
use crate::generated::RGB;

const FIRST_STEP: u8 = 10; // frames per LED at the start: 1.7 s to the centre
const LAST_STEP: u8 = 3;
const HIT_FX: u8 = 12;

pub struct Whack {
    arm: u8,
    lit: u8, // 0: waiting; else how many of its line are lit (from outside)
    fx: u8,  // frames of the hit flash
    step: u8,
    wait: u16,
    t: u8,
    hits: u16,
    score: u16,
    demo_at: u8,
}

impl Whack {
    fn next_mole(&mut self, rng: &mut Rng) {
        self.wait = (FPS / 2 + rng.next() % FPS) as u16; // 0.5 to 1.5 s
        self.arm = ((self.arm as u32 + 1 + rng.next() % 4) % 5) as u8;
        self.lit = 0;
        self.t = 0;
        self.demo_at = (1 + rng.next() % 3) as u8;
    }
}

impl Play for Whack {
    fn new(rng: &mut Rng) -> Self {
        let mut w =
            Self { arm: (rng.next() % 5) as u8, lit: 0, fx: 0, step: FIRST_STEP, wait: 0, t: 0, hits: 0, score: 0, demo_at: 1 };
        w.next_mole(rng);
        w
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        self.fx = self.fx.saturating_sub(1);
        let stop = if demo { self.lit > 0 && self.lit >= self.demo_at } else { inp.pressed != 0 };
        if self.lit == 0 {
            if stop && self.fx == 0 {
                return false; // too early (a double press on a hit does not count)
            }
            if self.wait > 0 {
                self.wait -= 1;
                if self.wait > 0 {
                    return true;
                }
            }
            self.lit = 1;
            return true;
        }
        if stop {
            self.score += 6 - self.lit as u16;
            self.hits += 1;
            if self.hits % 3 == 0 && self.step > LAST_STEP {
                self.step -= 1;
            }
            self.fx = HIT_FX;
            self.next_mole(rng);
            return true;
        }
        self.t += 1;
        if self.t >= self.step {
            self.t = 0;
            self.lit += 1;
            if self.lit as usize >= ARM_LINE {
                return false; // it reached the centre
            }
        }
        true
    }

    fn draw(&self, f: &mut Frame, _t: u32) {
        draw_number(f, self.score, 5, 255);
        for a in 0..5 {
            arm_sides(f, a, Rgb(40, 8, 0));
        }
        for k in 0..self.lit as usize {
            let c = if k + 1 == self.lit as usize { Rgb(255, 40, 0) } else { Rgb(120, 10, 0) };
            arm_line(f, self.arm, ARM_LINE - 1 - k, c);
        }
        if self.fx > 0 {
            // caught: the arms flash green
            for (i, l) in RGB.iter().enumerate() {
                if l.rank >= 3 {
                    f.rgb(i, Rgb(0, 255, 60).scaled((self.fx as u32 * 255 / HIT_FX as u32) as u8));
                }
            }
        }
    }

    fn score(&self) -> u16 {
        self.score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_hands_reach_level_5_and_an_early_press_ends_it() {
        let mut rng = Rng::new(5);
        let mut g = Whack::new(&mut rng);
        while g.score < 100 {
            let inp = Input { pressed: if g.lit == 1 { 1 } else { 0 }, held: 0 };
            assert!(g.tick(false, inp, &mut rng));
        }
        while g.fx > 0 || g.lit > 0 {
            g.tick(false, Input::default(), &mut rng);
        }
        assert!(!g.tick(false, Input { pressed: 1, held: 0 }, &mut rng), "too early");
    }
}
