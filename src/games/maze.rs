//! Maze; same as `c/src/games/maze.c`: you stay in the middle of the grid
//! and the maze moves around you; only the walls close by are lit, so you
//! see one or two corridors ahead. ◀ / ▶ go left / right, B up, A down
//! (held, they repeat). The arm pointing towards the exit lights up, and
//! the outline shows the time left. Five mazes, from 5 x 5 to 13 x 13
//! cells; each one solved is a level of challenge 0 (challenge.rs).

use crate::anim::FPS;
use crate::frame::{draw_number, sin8, Frame, Rgb, Rng};
use crate::game::{arm_line, arm_sides, Input, Play, ARM_LINE};
use crate::generated::{EDGE, GRID_H, GRID_W, RGB};

const MAZES: u8 = 5;
const CELLS: usize = 13; // the biggest maze, cells a side
const PX: usize = 2 * CELLS + 1; // the same in pixels: walls between cells
const ME_R: i32 = 7; // where the player stands on the grid
const ME_C: i32 = 8;
const REPEAT_AFTER: u8 = 8;
const REPEAT_EVERY: u8 = 4;
const SOLVED_FX: u16 = FPS as u16;

const DR: [i32; 4] = [0, 1, 0, -1]; // right, down, left, up
const DC: [i32; 4] = [1, 0, -1, 0];
const KEY_DIR: [usize; 4] = [2, 3, 1, 0]; // ◀ left, B up, A down, ▶ right

pub struct Maze {
    wall: [u32; PX], // bit c of row r: pixel (r, c) is a wall
    size: u8,        // this maze's pixels a side
    r: u8,           // the player, in pixels (odd)
    c: u8,
    solved: u8,
    dir: u8, // the demo's heading
    rep: u8, // frames a direction has been held
    time: u16,
    limit: u16,
    fx: u16, // frames of the "solved" flash
}

impl Maze {
    fn wall(&self, r: i32, c: i32) -> bool {
        if r < 0 || c < 0 || r >= self.size as i32 || c >= self.size as i32 {
            return false;
        }
        self.wall[r as usize] >> c & 1 != 0
    }

    fn open(&mut self, r: i32, c: i32) {
        self.wall[r as usize] &= !(1 << c);
    }

    /// A perfect maze (one way between any two cells): depth-first, carving.
    fn make(&mut self, rng: &mut Rng) {
        let n = 5 + 2 * self.solved as i32;
        self.size = (2 * n + 1) as u8;
        for (r, w) in self.wall.iter_mut().enumerate() {
            *w = if r < self.size as usize { (1 << self.size) - 1 } else { 0 };
        }
        let mut stack = [0u8; CELLS * CELLS];
        let mut top = 1;
        self.open(1, 1);
        while top > 0 {
            let cell = stack[top - 1] as i32;
            let (cr, cc) = (cell / n, cell % n);
            let mut options = [0usize; 4];
            let mut k = 0;
            for d in 0..4 {
                let (nr, nc) = (cr + DR[d], cc + DC[d]);
                if nr >= 0 && nc >= 0 && nr < n && nc < n && self.wall(2 * nr + 1, 2 * nc + 1) {
                    options[k] = d;
                    k += 1;
                }
            }
            if k == 0 {
                top -= 1;
                continue;
            }
            let d = options[(rng.next() % k as u32) as usize];
            self.open(2 * cr + 1 + DR[d], 2 * cc + 1 + DC[d]);
            self.open(2 * (cr + DR[d]) + 1, 2 * (cc + DC[d]) + 1);
            stack[top] = ((cr + DR[d]) * n + cc + DC[d]) as u8;
            top += 1;
        }
        self.r = 1;
        self.c = 1;
        self.dir = 0;
        self.limit = (15 + 8 * self.solved as u16) * FPS as u16;
        self.time = self.limit;
    }

    fn step(&mut self, d: usize) -> bool {
        if self.wall(self.r as i32 + DR[d], self.c as i32 + DC[d]) {
            return false;
        }
        self.r = (self.r as i32 + 2 * DR[d]) as u8;
        self.c = (self.c as i32 + 2 * DC[d]) as u8;
        true
    }
}

impl Play for Maze {
    const USES_EDGE: bool = true;

    fn new(rng: &mut Rng) -> Self {
        let mut m = Self { wall: [0; PX], size: 0, r: 1, c: 1, solved: 0, dir: 0, rep: 0, time: 0, limit: 1, fx: 0 };
        m.make(rng);
        m
    }

    fn won(&self) -> bool {
        self.solved >= MAZES
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        if self.fx > 0 {
            self.fx -= 1;
            if self.fx == 0 {
                self.make(rng);
            }
            return true;
        }
        if self.time > 0 {
            self.time -= 1;
            if self.time == 0 {
                return false;
            }
        }
        if demo {
            // keep a hand on the right-hand wall
            if self.time % 5 == 0 {
                for k in 0..4 {
                    let d = (self.dir as usize + 1 + 3 * k) % 4; // right, straight, left, back
                    if self.step(d) {
                        self.dir = d as u8;
                        break;
                    }
                }
            }
        } else {
            let mut key = (0..4).find(|&k| inp.pressed >> k & 1 != 0);
            if key.is_some() {
                self.rep = 0;
            } else {
                key = (0..4).find(|&k| inp.held >> k & 1 != 0);
                if key.is_some() {
                    self.rep = self.rep.wrapping_add(1);
                    if !(self.rep >= REPEAT_AFTER && self.rep % REPEAT_EVERY == 0) {
                        key = None;
                    }
                }
            }
            if let Some(k) = key {
                self.step(KEY_DIR[k]);
            }
        }
        if self.r == self.size - 2 && self.c == self.size - 2 {
            self.solved += 1;
            if self.solved >= MAZES {
                return false;
            }
            self.fx = SOLVED_FX;
        }
        true
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        if self.fx > 0 {
            draw_number(f, self.solved as u16, 5, 255);
            for i in 0..RGB.len() {
                f.rgb(i, Rgb(0, 200, 60));
            }
            return;
        }
        let exit = self.size as i32 - 2; // the far corner
        let (pr, pc) = (self.r as i32, self.c as i32);
        for r in 0..GRID_H as i32 {
            for c in 0..GRID_W as i32 {
                let (mr, mc) = (pr + r - ME_R, pc + c - ME_C);
                let d = (r - ME_R).abs().max((c - ME_C).abs());
                if self.wall(mr, mc) && d <= 4 {
                    f.grid(r as usize, c as usize, if d <= 2 { 255 } else if d == 3 { 110 } else { 35 });
                } else if mr == exit && mc == exit && d <= 5 {
                    f.grid(r as usize, c as usize, if t % 10 < 5 { 255 } else { 60 });
                }
            }
        }
        f.grid(ME_R as usize, ME_C as usize, if t % 16 < 12 { 255 } else { 90 });
        // compass: the arm that points most towards the exit
        let (dx, dy) = (exit - pc, exit - pr);
        let (mut best, mut best_dot) = (0u8, i32::MIN);
        for a in 0..5u8 {
            let ang = (a as u32 * 256 / 5) as u8;
            let dot = dx * (sin8(ang) as i32 - 128) - dy * (sin8(ang.wrapping_add(64)) as i32 - 128);
            if dot > best_dot {
                (best, best_dot) = (a, dot);
            }
        }
        for a in 0..5u8 {
            arm_sides(f, a, if a < self.solved { Rgb(0, 160, 40) } else { Rgb(10, 10, 30) });
        }
        for k in 0..ARM_LINE {
            arm_line(f, best, k, Rgb(60, 120, 255).scaled((80 + 35 * k) as u8));
        }
        // time left, round the outline
        let left = self.time as usize * EDGE.len() / self.limit as usize;
        for (j, &s) in EDGE.iter().enumerate() {
            let v = if j >= left {
                0
            } else if self.time < (5 * FPS) as u16 && t % 10 < 5 {
                20
            } else {
                70
            };
            f.set(s, v);
        }
    }

    fn score(&self) -> u16 {
        self.solved as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wall_follower_solves_all_five_in_time() {
        let mut rng = Rng::new(11);
        let mut g = Maze::new(&mut rng);
        let key = [3u8, 2, 0, 1]; // direction -> button: right ▶, down A, left ◀, up B
        let mut dir = 0usize;
        for i in 0.. {
            let mut inp = Input::default();
            if g.fx == 0 && i % 3 == 0 {
                for k in 0..4 {
                    let d = (dir + 1 + 3 * k) % 4;
                    if !g.wall(g.r as i32 + DR[d], g.c as i32 + DC[d]) {
                        dir = d;
                        inp.pressed = 1 << key[d];
                        break;
                    }
                }
            }
            if !g.tick(false, inp, &mut rng) {
                break;
            }
        }
        assert!(g.won(), "solved {} of 5", g.solved);
    }
}
