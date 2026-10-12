//! Pong against the badge: your paddle on the left edge (◀ up, ▶ down,
//! hold to keep moving), the badge's on the right. First to 5 wins. The
//! left arms show your points in blue, the right arms the badge's in red.

use crate::frame::{on_grid, Frame, Rgb, Rng};
use crate::game::{approach, arm_fill, Input, Play, BTN_LEFT, BTN_RIGHT};
use crate::generated::GRID_W;

const ONE: i32 = 256;
const RIGHT: i32 = GRID_W as i32 - 1;
const TOP: i32 = 5; // paddles move over rows 5..=10, 2 tall
const BOTTOM: i32 = 9;
const WIN: u8 = 5;
const BLUE: Rgb = Rgb(20, 90, 255);
const RED: Rgb = Rgb(255, 20, 20);

pub struct Pong {
    left: i32, // top row of each paddle
    right: i32,
    y: i32,
    x: i32,
    vy: i32,
    vx: i32,
    points: [u8; 2],
    serve: u8,
    aim_err: [i32; 2], // the computer players miss on purpose sometimes
    flash: (u8, u8),   // (frames, who scored)
    t: u32,
}

fn cell(v: i32) -> i32 {
    (v + ONE / 2) >> 8
}

impl Pong {
    fn serve(&mut self, towards: i32, rng: &mut Rng) {
        self.y = 7 * ONE + ONE / 2;
        self.x = 8 * ONE;
        self.vx = towards * ONE * 2 / 7;
        self.vy = if rng.next() & 1 == 0 { ONE / 8 } else { -ONE / 8 };
        self.serve = 20;
        self.aim_err = [0, 0];
    }

    /// Where a computer paddle wants its top row.
    fn aim(&self, side: usize) -> i32 {
        let coming = if side == 0 { self.vx < 0 } else { self.vx > 0 };
        let y = if coming { self.y + self.aim_err[side] } else { 7 * ONE };
        (cell(y) - 1).clamp(TOP, BOTTOM)
    }
}

impl Play for Pong {
    fn new(rng: &mut Rng) -> Self {
        let mut p = Self {
            left: 7,
            right: 7,
            y: 0,
            x: 0,
            vy: 0,
            vx: 0,
            points: [0; 2],
            serve: 0,
            aim_err: [0; 2],
            flash: (0, 0),
            t: 0,
        };
        p.serve(1, rng);
        p
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        self.t += 1;
        self.flash.0 = self.flash.0.saturating_sub(1);
        let t = self.t;

        if demo {
            let aim = self.aim(0);
            approach(&mut self.left, aim, t, 3);
        } else if inp.pressed & BTN_LEFT != 0 || inp.held & BTN_LEFT != 0 && t % 3 == 0 {
            self.left = (self.left - 1).max(TOP);
        } else if inp.pressed & BTN_RIGHT != 0 || inp.held & BTN_RIGHT != 0 && t % 3 == 0 {
            self.left = (self.left + 1).min(BOTTOM);
        }
        let aim = self.aim(1);
        approach(&mut self.right, aim, t + 1, 3);

        if self.serve > 0 {
            self.serve -= 1;
            return true;
        }
        let (ny, nx) = (self.y + self.vy, self.x + self.vx);
        if !on_grid(cell(ny), cell(self.x).clamp(0, RIGHT)) {
            self.vy = -self.vy;
        }
        for (side, top, col) in [(0, self.left, 1), (1, self.right, RIGHT - 1)] {
            let moving = if side == 0 { self.vx < 0 } else { self.vx > 0 };
            let line = col * ONE;
            let crosses = (self.x - line) * (nx - line) <= 0;
            let covered = self.y >= top * ONE - ONE * 3 / 5 && self.y <= (top + 1) * ONE + ONE * 3 / 5;
            if moving && crosses && covered {
                self.vx = -self.vx * 26 / 25;
                self.vy = (self.vy + (self.y - top * ONE - ONE / 2) / 10).clamp(-ONE / 3, ONE / 3);
                // the other side's computer player may misjudge this one
                self.aim_err[1 - side] = if rng.next() % 4 == 0 { ONE * 5 / 2 } else { 0 };
            }
        }
        self.y += self.vy;
        self.x += self.vx;

        if self.x < -ONE / 2 || self.x > RIGHT * ONE + ONE / 2 {
            let who = if self.x < 0 { 1 } else { 0 };
            self.points[who] += 1;
            self.flash = (16, who as u8);
            if self.points[who] == WIN {
                return false;
            }
            self.serve(if who == 0 { 1 } else { -1 }, rng);
        }
        true
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        for r in (0..16).step_by(2) {
            f.grid(r, 8, 18); // the net
        }
        for d in 0..2 {
            f.grid((self.left + d) as usize, 0, 255);
            f.grid((self.right + d) as usize, RIGHT as usize, 255);
        }
        if self.serve == 0 || t % 6 < 3 {
            f.grid(cell(self.y).clamp(0, 15) as usize, cell(self.x).clamp(0, RIGHT) as usize, 255);
        }
        // left arms (3, 4) are yours, right arms (1, 2) the badge's
        for (who, arms, c) in [(0usize, [4u8, 3], BLUE), (1, [1, 2], RED)] {
            let flashing = self.flash.0 > 0 && self.flash.1 as usize == who;
            for a in arms {
                if flashing {
                    arm_fill(f, a, 7, c.scaled(self.flash.0 * 15));
                } else {
                    arm_fill(f, a, self.points[who] as i32, c.scaled(220));
                }
            }
        }
        arm_fill(f, 0, 7, Rgb(255, 255, 255).scaled(25));
    }

    fn score(&self) -> u16 {
        self.points[0] as u16
    }

    fn won(&self) -> bool {
        self.points[0] == WIN
    }
}
