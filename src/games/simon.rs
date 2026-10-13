//! Simon says; same as `c/src/games/simon.c`: the badge plays a sequence
//! on the four legs that have a button (◀ lower left, ▶ lower right, A
//! and B on the back of the upper arms), and you press it back. Every
//! round adds one. The top arm shows how far you are, and flashes green
//! when you got it all. A wrong button, or 5 s without one, ends it. The
//! grid shows the length. Challenge 1 (challenge.rs).

use crate::anim::FPS;
use crate::frame::{draw_number, Frame, Rgb, Rng};
use crate::game::{arm_line, arm_sides, Input, Play, ARM_LINE, BUTTON_ARM};

const MAX: usize = 32;
const GAP: u16 = 5;
const GOOD_FRAMES: u16 = 18;
const PRESS_FRAMES: u8 = 8;
const IDLE_MAX: u16 = (5 * FPS) as u16;

// classic Simon colours, by the arm (BUTTON_ARM): 1 yellow, 2 red, 3 blue, 4 green
const COLOUR: [Rgb; 5] = [Rgb(255, 255, 255), Rgb(255, 200, 0), Rgb(255, 20, 0), Rgb(0, 60, 255), Rgb(0, 255, 40)];

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Show,
    Input,
    Good,
}

pub struct Simon {
    seq: [u8; MAX], // buttons, 0..3
    len: u8,
    pos: u8, // position shown or entered
    state: State,
    lit: Option<u8>, // the button lit, and for how long
    lit_t: u8,
    t: u16,
    idle: u16,
    demo_slip: u8, // the demo gets this length wrong
    score: u8,
}

impl Simon {
    fn on_frames(&self) -> u16 {
        if self.len < 16 { 14 - self.len as u16 / 2 } else { 6 }
    }

    fn add_step(&mut self, rng: &mut Rng) {
        if (self.len as usize) < MAX {
            self.seq[self.len as usize] = (rng.next() % 4) as u8;
            self.len += 1;
        }
        self.state = State::Show;
        self.pos = 0;
        self.t = 0;
    }

    fn light(&mut self, b: u8, frames: u8) {
        self.lit = Some(b);
        self.lit_t = frames;
    }
}

impl Play for Simon {
    fn new(rng: &mut Rng) -> Self {
        let mut s = Self {
            seq: [0; MAX],
            len: 0,
            pos: 0,
            state: State::Show,
            lit: None,
            lit_t: 0,
            t: 0,
            idle: 0,
            demo_slip: (3 + rng.next() % 8) as u8,
            score: 0,
        };
        s.add_step(rng);
        s
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        self.t += 1;
        if self.lit_t > 0 {
            self.lit_t -= 1;
            if self.lit_t == 0 {
                self.lit = None;
            }
        }
        match self.state {
            State::Show => {
                let step = self.on_frames() + GAP;
                if self.t == (FPS / 2) as u16 + self.pos as u16 * step + 1 && self.pos < self.len {
                    let on = self.on_frames() as u8;
                    self.light(self.seq[self.pos as usize], on);
                    self.pos += 1;
                } else if self.pos == self.len && self.lit_t == 0 && self.t > (FPS / 2) as u16 {
                    self.state = State::Input;
                    self.pos = 0;
                    self.idle = 0;
                }
                true
            }
            State::Input => {
                let want = self.seq[self.pos as usize];
                let b = if demo {
                    self.idle += 1;
                    if self.idle < 12 {
                        None
                    } else if self.len == self.demo_slip && self.pos == self.len - 1 {
                        Some((want + 1) % 4)
                    } else {
                        Some(want)
                    }
                } else {
                    let b = (0..4u8).find(|&k| inp.pressed >> k & 1 != 0);
                    if b.is_none() {
                        self.idle += 1;
                        if self.idle >= IDLE_MAX {
                            return false;
                        }
                    }
                    b
                };
                let Some(b) = b else { return true };
                self.idle = 0;
                self.light(b, PRESS_FRAMES);
                if b != want {
                    return false;
                }
                self.pos += 1;
                if self.pos == self.len {
                    self.score = self.len;
                    self.state = State::Good;
                    self.t = 0;
                }
                true
            }
            State::Good => {
                if self.t >= GOOD_FRAMES {
                    self.add_step(rng);
                }
                true
            }
        }
    }

    fn draw(&self, f: &mut Frame, _t: u32) {
        draw_number(f, self.len as u16, 5, if self.state == State::Show { 80 } else { 255 });
        for (b, &arm) in BUTTON_ARM.iter().enumerate() {
            let on = self.lit == Some(b as u8);
            arm_sides(f, arm, COLOUR[arm as usize].scaled(if on { 255 } else { 50 }));
            if on {
                for k in 0..ARM_LINE {
                    arm_line(f, arm, k, COLOUR[arm as usize]);
                }
            }
        }
        match self.state {
            State::Good => {
                for k in 0..ARM_LINE {
                    arm_line(f, 0, k, Rgb(0, 255, 60));
                }
            }
            State::Input => {
                // how far into the sequence
                for k in 0..ARM_LINE {
                    let done = k < self.pos as usize * ARM_LINE / self.len as usize;
                    arm_line(f, 0, k, if done { Rgb(200, 200, 200) } else { Rgb(20, 20, 20) });
                }
            }
            State::Show => {}
        }
    }

    fn score(&self) -> u16 {
        self.score as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeating_the_sequence_reaches_level_5() {
        let mut rng = Rng::new(3);
        let mut g = Simon::new(&mut rng);
        let mut frames = 0;
        while g.score < 12 {
            let inp = if g.state == State::Input && g.idle > 3 {
                Input { pressed: 1 << g.seq[g.pos as usize], held: 0 }
            } else {
                Input::default()
            };
            assert!(g.tick(false, inp, &mut rng), "the right buttons never end it");
            frames += 1;
            assert!(frames < 100_000);
        }
        while g.state != State::Input {
            g.tick(false, Input::default(), &mut rng);
        }
        let wrong = (g.seq[0] + 1) % 4;
        assert!(!g.tick(false, Input { pressed: 1 << wrong, held: 0 }, &mut rng), "a wrong one does");
    }
}
