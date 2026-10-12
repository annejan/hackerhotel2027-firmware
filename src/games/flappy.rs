//! Flappy: one button. Touch T1, T2 or T4 to flap through the gaps. The
//! arms fill gold with the pipes you pass.

use crate::frame::{on_grid, Frame, Rgb, Rng};
use crate::game::{arms_score, Input, Play, T1, T2, T4};
use crate::generated::{GRID_H, GRID_W};

const ONE: i32 = 256;
const BIRD_COL: i32 = 4;
const GAP: i32 = 4; // rows
const PIPES: usize = 4;
const GRAVITY: i32 = 6;
const FLAP: i32 = -72;
const MAX_FALL: i32 = 100;
const SPEED: i32 = 57; // columns per frame, 1/256: about 6.7 per second
const EVERY: u32 = 36; // frames between pipes

#[derive(Clone, Copy)]
struct Pipe {
    x: i32, // column, fixed point; < 0 = unused
    gap: i32,
    passed: bool,
}

pub struct Flappy {
    y: i32,
    vy: i32,
    pipes: [Pipe; PIPES],
    score: u16,
    t: u32,
}

fn cell(v: i32) -> i32 {
    (v + ONE / 2) >> 8
}

impl Flappy {
    fn next_gap(&self) -> i32 {
        self.pipes
            .iter()
            .filter(|p| p.x >= 0 && p.x + ONE / 2 >= BIRD_COL * ONE)
            .min_by_key(|p| p.x)
            .map_or(7, |p| p.gap)
    }
}

impl Play for Flappy {
    fn new(_rng: &mut Rng) -> Self {
        Self { y: 7 * ONE, vy: 0, pipes: [Pipe { x: -1, gap: 0, passed: false }; PIPES], score: 0, t: 0 }
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        self.t += 1;
        if self.t % EVERY == 1 {
            if let Some(p) = self.pipes.iter_mut().find(|p| p.x < 0) {
                *p = Pipe { x: (GRID_W as i32 + 1) * ONE, gap: 3 + (rng.next() % 6) as i32, passed: false };
            }
        }
        for p in self.pipes.iter_mut().filter(|p| p.x >= 0) {
            p.x -= SPEED;
            if p.x < 0 {
                p.x = -1;
            }
            if !p.passed && p.x < BIRD_COL * ONE - ONE / 2 {
                p.passed = true;
                self.score += 1;
            }
        }

        let flap = if demo {
            // flap when about to sink below the middle of the next gap
            let mid = self.next_gap() * ONE + GAP * ONE / 2 - ONE / 2;
            self.vy >= 0 && self.y + self.vy * 3 > mid + ONE * 2 / 5
        } else {
            inp.pressed & (T1 | T2 | T4) != 0
        };
        if flap {
            self.vy = FLAP;
        }
        self.vy = (self.vy + GRAVITY).min(MAX_FALL);
        self.y += self.vy;

        let r = cell(self.y);
        let hit = self.pipes.iter().any(|p| {
            p.x >= 0 && (p.x - BIRD_COL * ONE).abs() < ONE * 3 / 5 && !(p.gap..p.gap + GAP).contains(&r)
        });
        !hit && on_grid(r, BIRD_COL)
    }

    fn draw(&self, f: &mut Frame, _t: u32) {
        for p in self.pipes.iter().filter(|p| p.x >= 0) {
            let c = cell(p.x);
            for r in 0..GRID_H as i32 {
                if !(p.gap..p.gap + GAP).contains(&r) && (0..GRID_W as i32).contains(&c) {
                    f.grid(r as usize, c as usize, 100);
                }
            }
        }
        let r = cell(self.y);
        if (0..GRID_H as i32).contains(&r) {
            f.grid(r as usize, BIRD_COL as usize, 255);
            let tail = (r + if self.vy < 0 { 1 } else { -1 }).clamp(0, GRID_H as i32 - 1);
            f.grid(tail as usize, BIRD_COL as usize - 1, 80);
        }
        arms_score(f, self.score, Rgb(255, 190, 20).scaled(220));
    }

    fn score(&self) -> u16 {
        self.score
    }
}
