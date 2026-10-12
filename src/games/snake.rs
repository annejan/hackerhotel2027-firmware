//! Snake on the 17 x 16 grid; the octagon's edge is the wall.
//! T1 turns left, T4 turns right. The demo steers by the shortest path to
//! the food. The arms fill green with the score.

use crate::frame::{on_grid, Frame, Rgb, Rng};
use crate::game::{arms_score, Input, Play, T1, T4};
use crate::generated::{EDGE, GRID_H, GRID_W};

const CELLS: usize = GRID_H * GRID_W;
const MAX_LEN: usize = 208; // every LED in the grid

// right, down, left, up; turning left is dir + 3, right is dir + 1
const DR: [i32; 4] = [0, 1, 0, -1];
const DC: [i32; 4] = [1, 0, -1, 0];

pub struct Snake {
    /// Ring buffer of cells (row * GRID_W + col), head at `head`.
    body: [u16; MAX_LEN],
    head: usize,
    len: usize,
    /// Occupied cells, one bit per column.
    occ: [u32; GRID_H],
    dir: usize,
    /// Turns pressed but not yet taken (+1 right, -1 left), at most two.
    turns: [i8; 2],
    nturns: usize,
    food: u16,
    score: u16,
    tick: u32,
    flash: u8,
}

fn rc(cell: u16) -> (i32, i32) {
    ((cell as usize / GRID_W) as i32, (cell as usize % GRID_W) as i32)
}

fn cell(r: i32, c: i32) -> u16 {
    (r as usize * GRID_W + c as usize) as u16
}

impl Snake {
    fn occupied(&self, cell: u16) -> bool {
        let (r, c) = rc(cell);
        self.occ[r as usize] >> c & 1 != 0
    }

    fn set(&mut self, cell: u16, on: bool) {
        let (r, c) = rc(cell);
        if on {
            self.occ[r as usize] |= 1 << c;
        } else {
            self.occ[r as usize] &= !(1 << c);
        }
    }

    fn push(&mut self, cell: u16) {
        self.head = (self.head + 1) % MAX_LEN;
        self.body[self.head] = cell;
        self.len += 1;
        self.set(cell, true);
    }

    fn tail(&self) -> u16 {
        self.body[(self.head + MAX_LEN + 1 - self.len) % MAX_LEN]
    }

    fn new_food(&self, rng: &mut Rng) -> u16 {
        let mut k = rng.next() as usize % (MAX_LEN - self.len).max(1);
        for r in 0..GRID_H as i32 {
            for c in 0..GRID_W as i32 {
                if on_grid(r, c) && !self.occupied(cell(r, c)) {
                    if k == 0 {
                        return cell(r, c);
                    }
                    k -= 1;
                }
            }
        }
        self.tail() // board full; never reached in practice
    }

    /// Demo: first step of a shortest path to the food, avoiding the body
    /// (the tail cell is fine, it moves away).
    fn autopilot(&self) -> usize {
        let head = self.body[self.head];
        let tail = self.tail();
        let mut from = [u8::MAX; CELLS]; // direction we came in with
        let mut queue = [0u16; MAX_LEN];
        let (mut qh, mut qt) = (0, 1);
        queue[0] = head;
        from[head as usize] = 4;
        while qh < qt {
            let p = queue[qh];
            qh += 1;
            if p == self.food {
                let mut p = p; // walk back to the cell next to the head
                loop {
                    let d = from[p as usize] as usize;
                    let (r, c) = rc(p);
                    let back = cell(r - DR[d], c - DC[d]);
                    if back == head {
                        return d;
                    }
                    p = back;
                }
            }
            let (r, c) = rc(p);
            for d in 0..4 {
                let (nr, nc) = (r + DR[d], c + DC[d]);
                if on_grid(nr, nc) {
                    let n = cell(nr, nc);
                    if from[n as usize] == u8::MAX && (!self.occupied(n) || n == tail) && qt < MAX_LEN {
                        from[n as usize] = d as u8;
                        queue[qt] = n;
                        qt += 1;
                    }
                }
            }
        }
        // no path: straight on if safe, else whichever turn is
        let (r, c) = rc(head);
        for d in [self.dir, (self.dir + 3) % 4, (self.dir + 1) % 4] {
            let (nr, nc) = (r + DR[d], c + DC[d]);
            if on_grid(nr, nc) && !self.occupied(cell(nr, nc)) {
                return d;
            }
        }
        self.dir
    }

    /// One move. Returns false on a crash.
    fn step(&mut self, demo: bool, rng: &mut Rng) -> bool {
        if demo {
            self.dir = self.autopilot();
        } else if self.nturns > 0 {
            self.dir = (self.dir as i32 + 4 + self.turns[0] as i32) as usize % 4;
            self.turns[0] = self.turns[1];
            self.nturns -= 1;
        }
        let (r, c) = rc(self.body[self.head]);
        let (nr, nc) = (r + DR[self.dir], c + DC[self.dir]);
        if !on_grid(nr, nc) {
            return false;
        }
        let next = cell(nr, nc);
        let eating = next == self.food;
        if self.occupied(next) && (eating || next != self.tail()) {
            return false;
        }
        if !eating {
            let tail = self.tail();
            self.set(tail, false);
            self.len -= 1;
        }
        self.push(next);
        if eating {
            self.score += 1;
            self.flash = 8;
            self.food = self.new_food(rng);
        }
        true
    }
}

impl Play for Snake {
    fn new(rng: &mut Rng) -> Self {
        let mut s = Self {
            body: [0; MAX_LEN],
            head: MAX_LEN - 1,
            len: 0,
            occ: [0; GRID_H],
            dir: 0,
            turns: [0; 2],
            nturns: 0,
            food: 0,
            score: 0,
            tick: 0,
            flash: 0,
        };
        for c in 4..=6 {
            s.push(cell(8, c));
        }
        s.food = s.new_food(rng);
        s
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        for (pad, turn) in [(T1, -1), (T4, 1)] {
            if inp.pressed & pad != 0 && self.nturns < 2 {
                self.turns[self.nturns] = turn;
                self.nturns += 1;
            }
        }
        self.tick += 1;
        self.flash = self.flash.saturating_sub(1);
        // 7.5 moves per second, 10 from a score of 10
        let every = if self.score >= 10 { 3 } else { 4 };
        self.tick % every != 0 || self.step(demo, rng)
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        for k in 0..self.len {
            let (r, c) = rc(self.body[(self.head + MAX_LEN - k) % MAX_LEN]);
            let v = if k == 0 { 255 } else { 200u32.saturating_sub(k as u32 * 5).max(90) as u8 };
            f.grid(r as usize, c as usize, v);
        }
        if t % 6 < 4 {
            let (r, c) = rc(self.food);
            f.grid(r as usize, c as usize, 230);
        }
        arms_score(f, self.score, Rgb(25, 255, 60).scaled(180));
        if self.flash > 0 {
            for i in 0..crate::generated::RGB.len() {
                f.rgb(i, Rgb(50, 255, 80).scaled(self.flash * 31));
            }
            for &s in EDGE.iter() {
                f.set(s, self.flash * 25);
            }
        }
    }

    fn score(&self) -> u16 {
        self.score
    }
}
