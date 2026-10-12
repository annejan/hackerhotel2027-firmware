//! Snake on the 17 x 16 grid; the octagon's edge is the wall.
//!
//! Attract mode plays by itself (shortest path to the food) until T4 starts
//! a game. In a game T1 turns left and T4 turns right; turns react on
//! touch-down (`Event::Press`). The arms fill green with the score.

use crate::anim::{FPS, MASK};
use crate::font;
use crate::frame::{hsv, Frame, Rgb, Rng};
use crate::generated::{EDGE, GRID_H, GRID_W, RGB};
use crate::Event;

const CELLS: usize = GRID_H * GRID_W;
const MAX_LEN: usize = 208; // every LED in the grid
const CRASH_FRAMES: u8 = 20;
const SCORE_FRAMES: u8 = (2 * FPS) as u8;

// right, down, left, up; turning left is dir + 3, right is dir + 1
const DR: [i32; 4] = [0, 1, 0, -1];
const DC: [i32; 4] = [1, 0, -1, 0];

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Attract,
    Play,
    Crash(u8),
    Score(u8),
}

pub struct Snake {
    state: State,
    /// This run is attract mode (including its score pause), not a game.
    demo: bool,
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

fn free(r: i32, c: i32) -> bool {
    (0..GRID_H as i32).contains(&r) && (0..GRID_W as i32).contains(&c) && MASK[r as usize] >> c & 1 != 0
}

fn rc(cell: u16) -> (i32, i32) {
    ((cell as usize / GRID_W) as i32, (cell as usize % GRID_W) as i32)
}

fn cell(r: i32, c: i32) -> u16 {
    (r as usize * GRID_W + c as usize) as u16
}

impl Snake {
    pub fn new(rng: &mut Rng) -> Self {
        let mut s = Self {
            state: State::Attract,
            demo: true,
            body: [0; MAX_LEN],
            head: 0,
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
        s.reset(rng, State::Attract);
        s
    }

    fn reset(&mut self, rng: &mut Rng, state: State) {
        self.state = state;
        self.demo = state == State::Attract;
        self.occ = [0; GRID_H];
        self.len = 0;
        self.head = MAX_LEN - 1;
        for c in 4..=6 {
            self.push(cell(8, c));
        }
        self.dir = 0;
        self.nturns = 0;
        self.score = 0;
        self.tick = 0;
        self.food = self.new_food(rng);
    }

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
        let free_cells = MAX_LEN - self.len;
        let mut k = rng.next() as usize % free_cells.max(1);
        for r in 0..GRID_H as i32 {
            for c in 0..GRID_W as i32 {
                if free(r, c) && !self.occupied(cell(r, c)) {
                    if k == 0 {
                        return cell(r, c);
                    }
                    k -= 1;
                }
            }
        }
        self.tail() // board full; never reached in practice
    }

    /// Attract mode: first step of a shortest path to the food, avoiding the
    /// body (the tail cell is fine, it moves away).
    fn autopilot(&self) -> usize {
        let head = self.body[self.head];
        let tail = self.tail();
        let mut from = [u8::MAX; CELLS]; // direction we came in with
        let mut queue = [0u16; MAX_LEN];
        let (mut qh, mut qt) = (0, 0);
        queue[qt] = head;
        qt += 1;
        from[head as usize] = 4;
        while qh < qt {
            let p = queue[qh];
            qh += 1;
            if p == self.food {
                // walk back to the cell next to the head
                let mut p = p;
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
                if free(nr, nc) {
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
            if free(nr, nc) && !self.occupied(cell(nr, nc)) {
                return d;
            }
        }
        self.dir
    }

    /// One move. Returns false on a crash.
    fn step(&mut self, rng: &mut Rng) -> bool {
        if self.state == State::Attract {
            self.dir = self.autopilot();
        } else if self.nturns > 0 {
            self.dir = (self.dir as i32 + 4 + self.turns[0] as i32) as usize % 4;
            self.turns[0] = self.turns[1];
            self.nturns -= 1;
        }
        let (r, c) = rc(self.body[self.head]);
        let (nr, nc) = (r + DR[self.dir], c + DC[self.dir]);
        if !free(nr, nc) {
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

    /// Returns true when the event was used by the game.
    pub fn handle(&mut self, ev: Event, rng: &mut Rng) -> bool {
        match (self.state, ev) {
            (_, Event::Tap(3)) if self.demo => {
                self.reset(rng, State::Play);
                true
            }
            (State::Play, Event::Press(p @ (0 | 3))) => {
                if self.nturns < 2 {
                    self.turns[self.nturns] = if p == 0 { -1 } else { 1 };
                    self.nturns += 1;
                }
                true
            }
            (_, Event::Tap(_)) => !self.demo,
            _ => false,
        }
    }

    pub fn playing(&self) -> bool {
        !self.demo
    }

    pub fn render(&mut self, f: &mut Frame, rng: &mut Rng) {
        self.tick += 1;
        match self.state {
            State::Attract | State::Play => {
                // 7.5 moves per second, 10 from a score of 10
                let every = if self.score >= 10 { 3 } else { 4 };
                if self.tick % every == 0 && !self.step(rng) {
                    self.state = if self.state == State::Play {
                        State::Crash(CRASH_FRAMES)
                    } else {
                        State::Score(SCORE_FRAMES / 2) // attract: short pause, then again
                    };
                }
            }
            State::Crash(0) => self.state = State::Score(SCORE_FRAMES),
            State::Crash(n) => self.state = State::Crash(n - 1),
            State::Score(0) => self.reset(rng, State::Attract),
            State::Score(n) => self.state = State::Score(n - 1),
        }

        if let State::Score(n) = self.state {
            draw_number(f, self.score, n % 10 < 7);
            for (i, l) in RGB.iter().enumerate() {
                let hue = (l.arm * 51).wrapping_add(l.rank * 18).wrapping_sub((self.tick * 6) as u8);
                f.rgb(i, hsv(hue, 255, 180));
            }
            return;
        }

        let crash = match self.state {
            State::Crash(n) => Some(n),
            _ => None,
        };
        let shown = crash.map_or(true, |n| n % 6 < 3);
        for k in 0..self.len {
            let (r, c) = rc(self.body[(self.head + MAX_LEN - k) % MAX_LEN]);
            let v = if k == 0 { 255 } else { 200u32.saturating_sub(k as u32 * 5).max(90) as u8 };
            if shown {
                f.grid(r as usize, c as usize, v);
            }
        }
        if crash.is_none() && self.tick % 6 < 4 {
            let (r, c) = rc(self.food);
            f.grid(r as usize, c as usize, 230);
        }

        // arms: score, 7 per arm; red while crashing
        for (i, l) in RGB.iter().enumerate() {
            let lit = (self.score as i32 - l.arm as i32 * 7).clamp(0, 7) > l.rank as i32;
            let c = match crash {
                Some(n) => Rgb(255, 10, 0).scaled(n * (255 / CRASH_FRAMES)),
                None if self.flash > 0 => Rgb(50, 255, 80).scaled(self.flash * 31),
                None if lit => Rgb(25, 255, 60).scaled(180),
                None => Rgb(0, 0, 0),
            };
            f.rgb(i, c);
        }
        // outline: flash on eating; a slow dim chase while it plays itself
        for (j, &s) in EDGE.iter().enumerate() {
            let v = if self.flash > 0 {
                self.flash * 25
            } else if self.state == State::Attract && (j as u32 + self.tick / 3) % 12 == 0 {
                60
            } else {
                0
            };
            f.set(s, v);
        }
        self.flash = self.flash.saturating_sub(1);
    }
}

/// Score in 3x5 digits, centred on the grid.
fn draw_number(f: &mut Frame, n: u16, on: bool) {
    let mut digits = [0u8; 5];
    let mut len = 0;
    let mut n = n;
    loop {
        digits[len] = (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 || len == digits.len() {
            break;
        }
    }
    let width = len * 4 - 1;
    let c0 = (GRID_W - width) / 2;
    for k in 0..len {
        let rows = font::digit3x5(digits[len - 1 - k]);
        for (r, bits) in rows.iter().enumerate() {
            for c in 0..3 {
                if bits >> (2 - c) & 1 != 0 {
                    f.grid(5 + r, c0 + k * 4 + c, if on { 255 } else { 70 });
                }
            }
        }
    }
}
