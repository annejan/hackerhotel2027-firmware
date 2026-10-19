//! Star Chase: a light runs round the outline LEDs. Stop it (◀, B or
//! ▶) beside the arm that is lit green. Every hit speeds it up; three
//! misses and it is over. The grid shows the level.

use crate::frame::{draw_number, hsv, polar, Frame, Rgb, Rng};
use crate::game::{arm_fill, Input, Play, BTN_LEFT, BTN_B, BTN_RIGHT};
use crate::generated::{EDGE, RGB};

const N: i32 = EDGE.len() as i32;
const ONE: i32 = 256;
const ZONE: i32 = 7; // 1/256 turn either side of the arm tip
const LIVES: u8 = 3;
const FX: u8 = 16;

pub struct Chase {
    pos: i32, // outline position, 1/256 LED
    speed: i32,
    arm: u8,
    hits: u16,
    lives: u8,
    fx: (u8, bool), // (frames, hit)
    t: u32,
}

fn angle_dist(a: u8, b: u8) -> i32 {
    let d = a.wrapping_sub(b) as i8;
    (d as i32).abs()
}

impl Chase {
    fn tip_angle(&self) -> u8 {
        let tip = RGB.iter().find(|l| l.arm == self.arm && l.rank == 6).unwrap();
        polar(tip.r).1
    }

    fn in_zone(&self, j: i32) -> bool {
        angle_dist(polar(EDGE[j as usize]).1, self.tip_angle()) <= ZONE
    }

    fn head(&self) -> i32 {
        (self.pos / ONE).rem_euclid(N)
    }
}

impl Play for Chase {
    const USES_EDGE: bool = true;

    fn new(rng: &mut Rng) -> Self {
        Self { pos: 0, speed: ONE * 3 / 5, arm: (rng.next() % 5) as u8, hits: 0, lives: LIVES, fx: (0, false), t: 0 }
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        self.t += 1;
        if self.fx.0 > 0 {
            self.fx.0 -= 1;
            return self.fx.0 > 0 || self.lives > 0;
        }
        self.pos = (self.pos + self.speed).rem_euclid(N * ONE);
        let head = self.head();
        let stop = if demo {
            // press near the end of the zone; from level 5 on, a bit late
            if self.hits < 4 {
                self.in_zone(head) && !self.in_zone((head + 2).rem_euclid(N))
            } else {
                !self.in_zone(head) && self.in_zone((head - 2).rem_euclid(N))
            }
        } else {
            inp.pressed & (BTN_LEFT | BTN_B | BTN_RIGHT) != 0
        };
        if stop {
            if self.in_zone(head) {
                self.hits += 1;
                self.speed = (self.speed * 6 / 5).min(ONE * 2); // never jump over a zone
                self.arm = (self.arm + 1 + (rng.next() % 4) as u8) % 5;
                self.fx = (FX, true);
            } else {
                self.lives -= 1;
                self.fx = (FX, false);
            }
        }
        true
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        let head = self.head();
        for j in 0..N {
            if self.in_zone(j) {
                f.set(EDGE[j as usize], 50);
            }
        }
        for k in 0..6 {
            f.set(EDGE[(head - k).rem_euclid(N) as usize], (255 >> k) as u8);
        }
        let (n, hit) = self.fx;
        if n > 0 && hit {
            for r in 0..16 {
                for c in 0..17 {
                    f.grid(r, c, n * 12);
                }
            }
            for (i, l) in RGB.iter().enumerate() {
                f.rgb(i, hsv((l.arm * 51).wrapping_add((t * 10) as u8), 255, n * 15));
            }
        } else if n > 0 {
            for i in 0..RGB.len() {
                f.rgb(i, Rgb(255, 0, 0).scaled(if n % 6 < 3 { 230 } else { 0 }));
            }
        } else {
            let pulse = 128 + crate::frame::sin8((t * 8) as u8) / 2;
            arm_fill(f, self.arm, 7, Rgb(20, 255, 60).scaled(pulse));
        }
        draw_number(f, self.hits + 1, 4, 200);
        for k in 0..self.lives as usize {
            f.grid(12, 6 + k * 2, 120);
        }
    }

    fn score(&self) -> u16 {
        self.hits
    }
}
