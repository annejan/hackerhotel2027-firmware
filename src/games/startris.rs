//! Startris: a 9-wide, 14-deep well in the middle of the octagon.
//! T1 left, T4 right (repeat when held), T3 rotate, T2 drop. The arms show
//! the next piece's colour and fill with cleared lines. The demo places
//! each piece where a simple height/holes/bumpiness score likes it best.

use crate::frame::{hsv, Frame, Rng};
use crate::game::{arms_score, Input, Play, T1, T2, T3, T4};
use crate::generated::RGB;

const W: i32 = 9;
const H: usize = 14;
const R0: usize = 1; // well top row on the grid
const C0: usize = 4; // well left column on the grid
const FULL: u16 = (1 << W) - 1;

const I: u8 = 0;
const O: u8 = 1;
/// Cells of each piece in its 3x3 box (I: 4x4); rotation turns the box.
const SHAPES: [[(i8, i8); 4]; 7] = [
    [(1, 0), (1, 1), (1, 2), (1, 3)], // I
    [(0, 0), (0, 1), (1, 0), (1, 1)], // O
    [(0, 1), (1, 0), (1, 1), (1, 2)], // T
    [(0, 1), (0, 2), (1, 0), (1, 1)], // S
    [(0, 0), (0, 1), (1, 1), (1, 2)], // Z
    [(0, 0), (1, 0), (1, 1), (1, 2)], // J
    [(0, 2), (1, 0), (1, 1), (1, 2)], // L
];
const HUE: [u8; 7] = [128, 40, 200, 85, 0, 165, 20];

fn cells(kind: u8, rot: u8) -> [(i32, i32); 4] {
    let n = if kind == I { 4 } else { 3 };
    let mut out = SHAPES[kind as usize].map(|(r, c)| (r as i32, c as i32));
    if kind != O {
        for _ in 0..rot % 4 {
            out = out.map(|(r, c)| (c, n - 1 - r));
        }
    }
    out
}

pub struct Startris {
    well: [u16; H], // bit c = column c filled
    kind: u8,
    rot: u8,
    r: i32,
    c: i32,
    next: u8,
    bag: u8, // pieces already dealt from the current 7-bag
    fall: u32,
    lines: u16,
    clearing: u16, // rows being cleared, bit per row
    clear_t: u8,
    rep: [u8; 2], // frames T1 / T4 held, for auto-repeat
    target: (u8, i32),
}

impl Startris {
    fn deal(&mut self, rng: &mut Rng) -> u8 {
        if self.bag == 0x7F {
            self.bag = 0;
        }
        loop {
            let k = (rng.next() % 7) as u8;
            if self.bag & 1 << k == 0 {
                self.bag |= 1 << k;
                return k;
            }
        }
    }

    fn fits_in(well: &[u16; H], kind: u8, rot: u8, r: i32, c: i32) -> bool {
        cells(kind, rot).iter().all(|&(dr, dc)| {
            let (rr, cc) = (r + dr, c + dc);
            (0..H as i32).contains(&rr) && (0..W).contains(&cc) && well[rr as usize] >> cc & 1 == 0
        })
    }

    fn fits(&self, rot: u8, r: i32, c: i32) -> bool {
        Self::fits_in(&self.well, self.kind, rot, r, c)
    }

    /// Remove the rows in `mask`, letting the rest fall. Returns how many
    /// were removed.
    fn compact(well: &mut [u16; H], mask: u16) -> u16 {
        let mut j = H;
        for i in (0..H).rev() {
            if mask >> i & 1 == 0 {
                j -= 1;
                well[j] = well[i];
            }
        }
        for row in well.iter_mut().take(j) {
            *row = 0;
        }
        j as u16
    }

    fn full_rows(well: &[u16; H]) -> u16 {
        (0..H).filter(|&i| well[i] == FULL).fold(0, |m, i| m | 1 << i)
    }

    /// How much the demo likes dropping the piece at (rot, c).
    fn rate(&self, rot: u8, c: i32) -> i32 {
        let mut r = 0;
        while self.fits(rot, r + 1, c) {
            r += 1;
        }
        let mut w = self.well;
        for (dr, dc) in cells(self.kind, rot) {
            w[(r + dr) as usize] |= 1 << (c + dc);
        }
        let full = Self::full_rows(&w);
        let lines = Self::compact(&mut w, full) as i32;
        let (mut agg, mut holes, mut bump, mut prev) = (0, 0, 0, -1);
        for x in 0..W {
            let top = (0..H).find(|&i| w[i] >> x & 1 != 0).unwrap_or(H);
            let height = (H - top) as i32;
            holes += (top..H).filter(|&i| w[i] >> x & 1 == 0).count() as i32;
            agg += height;
            if prev >= 0 {
                bump += (height - prev).abs();
            }
            prev = height;
        }
        76 * lines - 51 * agg - 36 * holes - 18 * bump
    }

    fn spawn(&mut self, rng: &mut Rng) -> bool {
        self.kind = self.next;
        self.next = self.deal(rng);
        self.rot = 0;
        self.r = 0;
        self.c = 3;
        if !self.fits(0, 0, 3) {
            return false;
        }
        let mut best = (i32::MIN, (0, 3));
        for rot in 0..if self.kind == O { 1 } else { 4 } {
            for c in -2..W {
                if self.fits(rot, 0, c) {
                    let v = self.rate(rot, c);
                    if v > best.0 {
                        best = (v, (rot, c));
                    }
                }
            }
        }
        self.target = best.1;
        true
    }

    fn rotate(&mut self) {
        let rot = (self.rot + 1) % 4;
        for kick in [0, -1, 1] {
            if self.fits(rot, self.r, self.c + kick) {
                self.rot = rot;
                self.c += kick;
                return;
            }
        }
    }

    fn shift(&mut self, d: i32) {
        if self.fits(self.rot, self.r, self.c + d) {
            self.c += d;
        }
    }

    /// Put the piece down; returns false when the next one has no room.
    fn lock(&mut self, rng: &mut Rng) -> bool {
        for (dr, dc) in cells(self.kind, self.rot) {
            self.well[(self.r + dr) as usize] |= 1 << (self.c + dc);
        }
        self.clearing = Self::full_rows(&self.well);
        if self.clearing != 0 {
            self.clear_t = 9;
            return true;
        }
        self.spawn(rng)
    }

    fn drop(&mut self, rng: &mut Rng) -> bool {
        while self.fits(self.rot, self.r + 1, self.c) {
            self.r += 1;
        }
        self.lock(rng)
    }
}

impl Play for Startris {
    fn new(rng: &mut Rng) -> Self {
        let mut s = Self {
            well: [0; H],
            kind: 0,
            rot: 0,
            r: 0,
            c: 3,
            next: 0,
            bag: 0,
            fall: 0,
            lines: 0,
            clearing: 0,
            clear_t: 0,
            rep: [0; 2],
            target: (0, 3),
        };
        s.next = s.deal(rng);
        s.spawn(rng);
        s
    }

    fn tick(&mut self, demo: bool, inp: Input, rng: &mut Rng) -> bool {
        if self.clear_t > 0 {
            self.clear_t -= 1;
            if self.clear_t == 0 {
                Self::compact(&mut self.well, self.clearing);
                self.lines += self.clearing.count_ones() as u16;
                self.clearing = 0;
                return self.spawn(rng);
            }
            return true;
        }
        if demo {
            self.fall += 1;
            if self.fall % 4 == 0 {
                // if the wanted rotation or move is blocked, drop where it is
                let (rot, c) = self.target;
                let before = (self.rot, self.c);
                if self.rot != rot {
                    self.rotate();
                } else if self.c != c {
                    self.shift(if c > self.c { 1 } else { -1 });
                }
                if (self.rot, self.c) == before {
                    return self.drop(rng);
                }
            }
            return true;
        }
        if inp.pressed & T3 != 0 {
            self.rotate();
        }
        for (k, pad, d) in [(0, T1, -1), (1, T4, 1)] {
            if inp.pressed & pad != 0 {
                self.shift(d);
                self.rep[k] = 0;
            } else if inp.held & pad != 0 {
                self.rep[k] = self.rep[k].saturating_add(1);
                if self.rep[k] > 8 && self.rep[k] % 3 == 0 {
                    self.shift(d);
                }
            }
        }
        if inp.pressed & T2 != 0 {
            return self.drop(rng);
        }
        self.fall += 1;
        if self.fall >= 20u32.saturating_sub(self.lines as u32).max(4) {
            self.fall = 0;
            if self.fits(self.rot, self.r + 1, self.c) {
                self.r += 1;
            } else {
                return self.lock(rng);
            }
        }
        true
    }

    fn draw(&self, f: &mut Frame, t: u32) {
        for r in 0..H {
            f.grid(R0 + r, C0 - 1, 25); // well walls
            f.grid(R0 + r, C0 + W as usize, 25);
            let blink = self.clearing >> r & 1 != 0;
            for c in 0..W as usize {
                if self.well[r] >> c & 1 != 0 {
                    f.grid(R0 + r, C0 + c, if !blink { 170 } else if self.clear_t % 3 < 2 { 255 } else { 40 });
                }
            }
        }
        if self.clear_t == 0 {
            for (dr, dc) in cells(self.kind, self.rot) {
                f.grid(R0 + (self.r + dr) as usize, C0 + (self.c + dc) as usize, 255);
            }
        }
        let next = hsv(HUE[self.next as usize], 255, 255);
        for i in 0..RGB.len() {
            f.rgb(i, next.scaled(40));
        }
        arms_score(f, self.lines, next.scaled(220));
        if self.clear_t > 0 {
            for (i, l) in RGB.iter().enumerate() {
                f.rgb(i, hsv((l.arm * 51).wrapping_add(l.rank * 9).wrapping_add((t * 12) as u8), 255, 255));
            }
        }
    }

    fn score(&self) -> u16 {
        self.lines
    }
}
