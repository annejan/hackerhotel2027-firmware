//! Breakout: five rows of bricks on top, a 3-wide paddle on the bottom row,
//! and the octagon's angled walls to bounce off. ◀ / ▶ move the paddle
//! (hold to keep moving). 3 lives on arms 0..2; the score fills orange.

use crate::frame::{on_grid, Frame, Rgb, Rng, MASK};
use crate::game::{approach, arm_fill, arms_score, Input, Play, BTN_LEFT, BTN_RIGHT};
use crate::generated::EDGE;

const ONE: i32 = 256; // fixed point: 1/256 cell
const BRICK_ROWS: usize = 5; // grid rows 1..=5
const PADDLE_ROW: i32 = 14; // grid columns 4..12 on this row
const SERVE_FRAMES: u8 = 20;

pub struct Breakout {
    bricks: [u32; BRICK_ROWS],
    paddle: i32, // centre column
    y: i32,
    x: i32,
    vy: i32,
    vx: i32,
    serve: u8, // frames the ball still sits on the paddle
    lives: u8,
    score: u16,
    speed: i32,
    flash: u8,
    t: u32,
}

fn cell(v: i32) -> i32 {
    (v + ONE / 2) >> 8
}

impl Breakout {
    fn fill(&mut self) {
        for (i, row) in self.bricks.iter_mut().enumerate() {
            *row = MASK[i + 1];
        }
    }

    fn brick(&self, r: i32, c: i32) -> bool {
        (1..=BRICK_ROWS as i32).contains(&r) && c >= 0 && self.bricks[r as usize - 1] >> c & 1 != 0
    }

    fn serve(&mut self) {
        self.serve = SERVE_FRAMES;
        self.vy = -self.speed;
        self.vx = self.speed / 3;
    }
}

impl Play for Breakout {
    fn new(_rng: &mut Rng) -> Self {
        let mut b = Self {
            bricks: [0; BRICK_ROWS],
            paddle: 8,
            y: 0,
            x: 0,
            vy: 0,
            vx: 0,
            serve: 0,
            lives: 3,
            score: 0,
            speed: ONE / 2, // 15 cells per second
            flash: 0,
            t: 0,
        };
        b.fill();
        b.serve();
        b
    }

    fn tick(&mut self, demo: bool, inp: Input, _rng: &mut Rng) -> bool {
        self.t += 1;
        self.flash = self.flash.saturating_sub(1);

        // paddle: the demo follows the ball, the player holds ◀ / ▶
        let target = if demo {
            cell(self.x)
        } else if inp.pressed & BTN_LEFT != 0 || inp.held & BTN_LEFT != 0 && self.t % 3 == 0 {
            self.paddle - 1
        } else if inp.pressed & BTN_RIGHT != 0 || inp.held & BTN_RIGHT != 0 && self.t % 3 == 0 {
            self.paddle + 1
        } else {
            self.paddle
        };
        approach(&mut self.paddle, target.clamp(5, 11), self.t, if demo { 2 } else { 1 });

        if self.serve > 0 {
            self.serve -= 1;
            self.y = (PADDLE_ROW - 1) * ONE;
            self.x = self.paddle * ONE;
            return true;
        }

        let (ny, nx) = (self.y + self.vy, self.x + self.vx);
        let (r, c) = (cell(ny), cell(nx));
        if self.brick(r, c) {
            self.bricks[r as usize - 1] &= !(1 << c);
            self.score += 1;
            self.flash = 6;
            self.vy = -self.vy;
        } else if r < PADDLE_ROW {
            if !on_grid(r, cell(self.x)) {
                self.vy = -self.vy;
            }
            if !on_grid(cell(self.y), c) {
                self.vx = -self.vx;
            }
        }
        // paddle: bounce, and steer by where the ball hits it
        let edge = PADDLE_ROW * ONE - ONE / 2;
        if self.vy > 0 && self.y < edge && self.y + self.vy >= edge && (self.x - self.paddle * ONE).abs() <= ONE * 8 / 5 {
            self.vy = -self.vy.abs();
            self.vx = (self.vx + (self.x - self.paddle * ONE) / 10).clamp(-self.speed * 3 / 5, self.speed * 3 / 5);
        }
        self.y += self.vy;
        self.x += self.vx;

        if self.y > (PADDLE_ROW + 1) * ONE + ONE / 2 {
            self.lives -= 1;
            if self.lives == 0 {
                return false;
            }
            self.serve();
        }
        if self.bricks.iter().all(|&r| r == 0) {
            self.fill(); // next wave, a bit faster
            self.speed += ONE / 16;
            self.serve();
        }
        true
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        for r in 0..BRICK_ROWS {
            for c in 0..17 {
                if self.bricks[r] >> c & 1 != 0 {
                    f.grid(r + 1, c, 130);
                }
            }
        }
        for d in -1..=1 {
            f.grid(PADDLE_ROW as usize, (self.paddle + d) as usize, 210);
        }
        if self.serve == 0 || t % 6 < 3 {
            f.grid(cell(self.y) as usize, cell(self.x) as usize, 255);
        }
        for a in 0..self.lives {
            arm_fill(f, a, 7, Rgb(20, 70, 255).scaled(70));
        }
        arms_score(f, self.score / 3, Rgb(255, 150, 20).scaled(220));
        for &s in EDGE.iter() {
            f.set(s, self.flash * 30);
        }
    }

    fn score(&self) -> u16 {
        self.score
    }
}
