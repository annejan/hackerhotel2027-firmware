//! Animations, ported from tools/sim/hh2027.py to integer maths.
//!
//! Controls: T1 next, T2 previous, T3 brightness, T4 restart/reseed,
//! hold any pad for night mode, hold T1 + T4 for 3 s to reboot into the USB
//! boot ROM. While charging, the battery gauge shows until the next tap.
//! Games take over the pads while one is being played (see snake.rs).

use crate::font;
use crate::frame::{hsv, mono_slots, polar, scale, sin8, Frame, Rgb, Rng};
use crate::generated::{EDGE, GRID, GRID_H, GRID_W, NONE, RGB};
use crate::snake::Snake;
use crate::Event;

pub const FPS: u32 = 30;

const BRIGHTNESS: [u8; 4] = [24, 64, 140, 255];
const NIGHT: u8 = 6;
const MESSAGE: &str = "HACKER HOTEL 2027 * ";

#[derive(Clone, Copy)]
enum Mode {
    Ripple,
    Galaxy,
    Heartbeat,
    Scroller,
    Life,
    Fire,
    Twinkle,
    Snake,
}

const MODES: [Mode; 8] = [
    Mode::Ripple,
    Mode::Galaxy,
    Mode::Heartbeat,
    Mode::Scroller,
    Mode::Life,
    Mode::Fire,
    Mode::Twinkle,
    Mode::Snake,
];

pub struct Player {
    mode: usize,
    t: u32,
    level: usize,
    night: bool,
    charging: bool,
    show_charge: bool,
    rng: Rng,
    life: Life,
    fire: Fire,
    snake: Snake,
}

impl Player {
    pub fn new() -> Self {
        let mut rng = Rng::new(0x2027_5EA5);
        let life = Life::new(&mut rng);
        let snake = Snake::new(&mut rng);
        Self {
            mode: 0,
            t: 0,
            level: 1,
            night: false,
            charging: false,
            show_charge: false,
            rng,
            life,
            fire: Fire::new(),
            snake,
        }
    }

    pub fn handle(&mut self, ev: Event) {
        let gaming = matches!(MODES[self.mode], Mode::Snake) && !self.show_charge;
        if gaming && self.snake.handle(ev, &mut self.rng) {
            return;
        }
        match ev {
            Event::Press(_) => {}
            Event::Tap(pad) => {
                self.show_charge = false;
                match pad {
                    0 => self.switch(self.mode + 1),
                    1 => self.switch(self.mode + MODES.len() - 1),
                    2 => self.level = (self.level + 1) % BRIGHTNESS.len(),
                    _ => self.switch(self.mode),
                }
            }
            Event::Hold(_) => self.night = !self.night,
            Event::Charging(c) => {
                self.charging = c;
                // don't cover a game that is being played
                self.show_charge = c && !(gaming && self.snake.playing());
            }
            Event::Bootloader => {} // handled by the main loop
        }
    }

    fn switch(&mut self, mode: usize) {
        self.mode = mode % MODES.len();
        self.t = 0;
        self.life = Life::new(&mut self.rng);
        self.fire = Fire::new();
        self.snake = Snake::new(&mut self.rng);
    }

    pub fn brightness(&self) -> u8 {
        if self.night {
            NIGHT
        } else {
            BRIGHTNESS[self.level]
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let t = self.t;
        self.t = self.t.wrapping_add(1);
        if self.show_charge {
            return charging(f, t);
        }
        match MODES[self.mode] {
            Mode::Ripple => ripple(f, t),
            Mode::Galaxy => galaxy(f, t),
            Mode::Heartbeat => heartbeat(f, t),
            Mode::Scroller => scroller(f, t),
            Mode::Life => self.life.render(f, t, &mut self.rng),
            Mode::Fire => self.fire.render(f, &mut self.rng),
            Mode::Twinkle => twinkle(f, t),
            Mode::Snake => self.snake.render(f, &mut self.rng),
        }
    }
}

/// Animation phase in 1/256 turns, advancing `per_s` units per second.
#[inline]
fn phase(t: u32, per_s: u32) -> u8 {
    (t.wrapping_mul(per_s) / FPS) as u8
}

fn ripple(f: &mut Frame, t: u32) {
    let w = phase(t, 171); // one ring every 1.5 s
    for s in mono_slots() {
        let (r, _) = polar(s);
        let v = sin8(((r as u32 * 6) as u8).wrapping_sub(w)); // 11 mm wavelength
        let v2 = scale(v, v);
        f.set(s, scale(v2, v2));
    }
    for (i, l) in RGB.iter().enumerate() {
        let (r, _) = polar(l.r);
        let v = sin8(((r as u32 * 6) as u8).wrapping_sub(w));
        let hue = ((r as u32 * 3 / 2) as u8).wrapping_sub(w / 2);
        f.rgb(i, hsv(hue, 255, 40 + scale(215, scale(v, v))));
    }
}

fn galaxy(f: &mut Frame, t: u32) {
    let w = phase(t, 85); // one turn every 3 s
    let arm = |r: u8, a: u8| (a as u32 * 5 + r as u32 * 5 / 2) as u8;
    for s in mono_slots() {
        let (r, a) = polar(s);
        let v = sin8(arm(r, a).wrapping_sub(w));
        let fall = 255u32.saturating_sub(r as u32 * 3 / 4).max(51) as u8;
        f.set(s, scale(scale(scale(v, v), v), fall));
    }
    for (i, l) in RGB.iter().enumerate() {
        let (r, a) = polar(l.r);
        let v = sin8(arm(r, a).wrapping_sub(w));
        f.rgb(i, hsv(a.wrapping_add(w), 255, 40 + scale(215, scale(v, v))));
    }
}

/// Double beat every 2.2 s, travelling outward at 120 mm/s.
fn heartbeat(f: &mut Frame, t: u32) {
    let ms = t * 1000 / FPS;
    let beat = |r: u8| -> u8 {
        let x = (ms + 2200 * 4 - r as u32 * 2) % 2200; // 2 ms per 0.25 mm
        let pulse = |c: u32, peak: u32| peak.saturating_sub(x.abs_diff(c) * peak / 150);
        (pulse(200, 255) + pulse(550, 180)).min(255) as u8
    };
    for s in mono_slots() {
        f.set(s, scale(beat(polar(s).0), 230));
    }
    for (i, l) in RGB.iter().enumerate() {
        let v = 15u8.saturating_add(beat(polar(l.r).0));
        f.rgb(i, Rgb(255, 13, 46).scaled(v));
    }
}

fn scroller(f: &mut Frame, t: u32) {
    let cols = MESSAGE.len() * (font::WIDTH + 1);
    let off = (t * 10 / FPS) as usize; // 10 columns per second
    for c in 0..GRID_W {
        let col = (off + c) % cols;
        let (ch, cx) = (col / (font::WIDTH + 1), col % (font::WIDTH + 1));
        let bits = if cx < font::WIDTH {
            font::glyph(MESSAGE.as_bytes()[ch] as char)[cx]
        } else {
            0
        };
        for r in 0..font::HEIGHT {
            if bits >> r & 1 != 0 {
                f.grid(4 + r, c, 255);
            }
        }
    }
    for (j, &s) in EDGE.iter().enumerate() {
        f.set(s, if (j + off) % 4 == 0 { 140 } else { 10 });
    }
    let w = phase(t, 128);
    for (i, l) in RGB.iter().enumerate() {
        let hue = (l.rank * 36).wrapping_add(l.arm * 51).wrapping_sub(w);
        f.rgb(i, hsv(hue, 255, 200));
    }
}

fn twinkle(f: &mut Frame, t: u32) {
    for s in mono_slots() {
        // per-LED speed and phase from a cheap hash of the slot
        let h = (s as u32).wrapping_mul(2_654_435_761);
        let speed = 40 + (h >> 28) * 12;
        let x = phase(t, speed).wrapping_add((h >> 8) as u8);
        let v = sin8(x);
        f.set(s, if v > 215 { ((v - 215) as u32 * 6).min(255) as u8 } else { 0 });
    }
    // a comet runs up each arm every 3 s, arms staggered
    for (i, l) in RGB.iter().enumerate() {
        let p = phase(t, 85).wrapping_add(l.arm * 51) as i32 * 11 * 16 / 256 - 32; // rank * 16
        let d = p - l.rank as i32 * 16;
        if (0..64).contains(&d) {
            let v = 255u32 >> (d / 16);
            f.rgb(i, hsv(140 + l.arm * 25, 150 + d as u8, v as u8));
        }
    }
}

/// Battery gauge on the grid, arms fill up. There is no battery voltage
/// sense on the board, so it only shows *that* it charges, not how full.
fn charging(f: &mut Frame, t: u32) {
    let ms = (t * 1000 / FPS) % 4000;
    let (r0, r1, c0, c1) = (4, 11, 2, 13);
    for c in c0..=c1 {
        f.grid(r0, c, 200);
        f.grid(r1, c, 200);
    }
    for r in r0..=r1 {
        f.grid(r, c0, 200);
        f.grid(r, c1, 200);
    }
    for r in 6..=9 {
        f.grid(r, c1 + 1, 200);
    }
    let cells = if ms < 3000 { (ms * 10 / 3000) as usize + 1 } else { 10 };
    for c in 0..cells {
        let blink = c + 1 < cells || ms >= 3000 || (ms / 150) % 2 == 0;
        for r in r0 + 2..=r1 - 2 {
            f.grid(r, c0 + 1 + c, if blink { 160 } else { 40 });
        }
    }
    let w = phase(t, 40);
    for (j, &s) in EDGE.iter().enumerate() {
        f.set(s, scale(60, sin8(w.wrapping_add((j * 256 / EDGE.len()) as u8))));
    }
    for (i, l) in RGB.iter().enumerate() {
        if (l.rank as usize) < cells * 7 / 10 {
            f.rgb(i, hsv((cells * 85 / 10) as u8, 255, 200));
        }
    }
}

/// Shown for a second before rebooting into the USB boot ROM: an arrow
/// moving down the grid (towards the USB port) and the arms filling blue.
pub fn usb_boot(f: &mut Frame, t: u32) {
    let shift = (t * 4 / FPS) as usize; // 4 rows per second
    for r in 0..9 {
        for c in 7..=9 {
            f.grid(r + shift, c, 220);
        }
    }
    for (i, w) in (0..5usize).zip([7, 5, 3, 1, 0]) {
        if w > 0 {
            for c in 8 - w / 2..=8 + w / 2 {
                f.grid(9 + i + shift, c, 255);
            }
        }
    }
    let fill = (t * 7 / FPS) as u8;
    for (i, l) in RGB.iter().enumerate() {
        if l.rank <= fill {
            f.rgb(i, Rgb(20, 60, 255));
        }
    }
}

// ------------------------------------------------------------------- life ---

/// Conway's Game of Life on the 17 x 16 grid, one bit per cell.
struct Life {
    cells: [u32; GRID_H],
    prev: [u32; GRID_H],
    seen: [u32; 2],
}

const fn grid_mask() -> [u32; GRID_H] {
    let mut m = [0u32; GRID_H];
    let mut r = 0;
    while r < GRID_H {
        let mut c = 0;
        while c < GRID_W {
            if GRID[r][c] != NONE {
                m[r] |= 1 << c;
            }
            c += 1;
        }
        r += 1;
    }
    m
}

/// Grid LEDs per row, one bit per column (the octagon).
pub(crate) const MASK: [u32; GRID_H] = grid_mask();

impl Life {
    fn new(rng: &mut Rng) -> Self {
        let mut l = Self { cells: [0; GRID_H], prev: [0; GRID_H], seen: [0; 2] };
        l.seed(rng);
        l
    }

    fn seed(&mut self, rng: &mut Rng) {
        for (row, mask) in self.cells.iter_mut().zip(MASK) {
            // ~34% alive: (a & b) is 25%, OR (c & d & e) adds another ~9%
            *row = (rng.next() & rng.next() | rng.next() & rng.next() & rng.next()) & mask;
        }
        self.prev = self.cells;
    }

    fn step(&mut self, rng: &mut Rng) {
        let mut next = [0u32; GRID_H];
        for r in 0..GRID_H {
            for c in 0..GRID_W {
                let mut n = 0;
                for dr in [-1i32, 0, 1] {
                    for dc in [-1i32, 0, 1] {
                        let (rr, cc) = (r as i32 + dr, c as i32 + dc);
                        if (dr, dc) != (0, 0)
                            && (0..GRID_H as i32).contains(&rr)
                            && (0..GRID_W as i32).contains(&cc)
                            && self.cells[rr as usize] >> cc & 1 != 0
                        {
                            n += 1;
                        }
                    }
                }
                let alive = self.cells[r] >> c & 1 != 0;
                if n == 3 || (alive && n == 2) {
                    next[r] |= 1 << c;
                }
            }
            next[r] &= MASK[r];
        }
        self.prev = self.cells;
        self.cells = next;

        // reseed when it dies out or settles into a still life / blinker
        let pop: u32 = self.cells.iter().map(|r| r.count_ones()).sum();
        let hash = self.cells.iter().fold(0x811C_9DC5u32, |h, &r| (h ^ r).wrapping_mul(0x0100_0193));
        if pop < 8 || self.seen.contains(&hash) {
            self.seed(rng);
            self.seen = [0; 2];
        } else {
            self.seen = [self.seen[1], hash];
        }
    }

    fn render(&mut self, f: &mut Frame, t: u32, rng: &mut Rng) {
        let gen_frames = FPS / 5; // 5 generations per second
        let k = t % gen_frames;
        if k == 0 && t > 0 {
            self.step(rng);
        }
        let mut births = 0;
        for r in 0..GRID_H {
            let born = self.cells[r] & !self.prev[r];
            let died = self.prev[r] & !self.cells[r];
            births += born.count_ones();
            for c in 0..GRID_W {
                let v = if born >> c & 1 != 0 {
                    255
                } else if self.cells[r] >> c & 1 != 0 {
                    200
                } else if died >> c & 1 != 0 {
                    // fade out over the generation
                    (120 - 120 * k / gen_frames) as u8
                } else {
                    0
                };
                f.grid(r, c, v);
            }
        }
        for &s in EDGE.iter() {
            f.set(s, 12);
        }
        let pop: u32 = self.cells.iter().map(|r| r.count_ones()).sum();
        let flash = (births * 20).min(200) * (gen_frames - k) / gen_frames;
        for (i, l) in RGB.iter().enumerate() {
            let hue = ((85 + pop * 2) as u8).wrapping_add(l.rank * 8);
            f.rgb(i, hsv(hue, 255, 30 + flash as u8));
        }
    }
}

// ------------------------------------------------------------------- fire ---

/// Doom-style fire: heat rises from two rows below the grid and cools.
struct Fire {
    heat: [[u8; GRID_W]; GRID_H + 2],
}

impl Fire {
    fn new() -> Self {
        Self { heat: [[0; GRID_W]; GRID_H + 2] }
    }

    fn render(&mut self, f: &mut Frame, rng: &mut Rng) {
        let h = GRID_H + 2;
        for x in 0..GRID_W {
            self.heat[h - 1][x] = 128 + rng.byte() / 2;
        }
        for y in 0..h - 1 {
            for x in 0..GRID_W {
                let shift = (rng.byte() % 3) as usize; // -1, 0, +1
                let src = self.heat[y + 1][(x + GRID_W + shift - 1) % GRID_W];
                self.heat[y][x] = src.saturating_sub(5 + rng.byte() % 34);
            }
        }
        for r in 0..GRID_H {
            for c in 0..GRID_W {
                let v = self.heat[r][c];
                f.grid(r, c, scale(v, v));
            }
        }
        // the lower half of the outline and arms glow like embers
        for &s in EDGE.iter() {
            let down = 255 - sin8(polar(s).1.wrapping_add(64));
            f.set(s, scale(down.saturating_sub(90), rng.byte() / 2 + 64));
        }
        for (i, l) in RGB.iter().enumerate() {
            let down = 255 - sin8(polar(l.r).1.wrapping_add(64));
            let v = scale(down, 100 + rng.byte() / 2);
            f.rgb(i, Rgb(255, 40 + v / 4, 4).scaled(v));
        }
    }
}
