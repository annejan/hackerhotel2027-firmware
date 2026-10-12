//! Frame buffer and small integer colour/maths helpers.

use crate::font;
use crate::generated::{Slot, EDGE, GRID, GRID_H, GRID_W, NONE, POLAR, RGB, SIN8, SLOTS};

/// One brightness byte per matrix slot (COM-major, see `generated.rs`), linear light.
pub struct Frame {
    pub px: [u8; SLOTS],
}

impl Frame {
    pub const fn new() -> Self {
        Self { px: [0; SLOTS] }
    }

    pub fn clear(&mut self) {
        self.px.fill(0);
    }

    #[inline]
    pub fn set(&mut self, s: Slot, v: u8) {
        if s != NONE {
            self.px[s as usize] = v;
        }
    }

    /// Grid pixel, row 0 = top. Out of range and cut-off corners are ignored.
    #[inline]
    pub fn grid(&mut self, row: usize, col: usize, v: u8) {
        if row < GRID_H && col < GRID_W {
            self.set(GRID[row][col], v);
        }
    }

    #[inline]
    pub fn rgb(&mut self, i: usize, c: Rgb) {
        let l = &RGB[i];
        self.set(l.r, c.0);
        self.set(l.g, c.1);
        self.set(l.b, c.2);
    }
}

/// Every mono LED (grid + outline), for effects that do not care which is which.
pub fn mono_slots() -> impl Iterator<Item = Slot> {
    GRID.iter()
        .flat_map(|row| row.iter().copied())
        .filter(|&s| s != NONE)
        .chain(EDGE.iter().copied())
}

/// (radius in 0.25 mm, angle in 1/256 turn) of a slot.
#[inline]
pub fn polar(s: Slot) -> (u8, u8) {
    POLAR[s as usize]
}

#[derive(Clone, Copy, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn scaled(self, v: u8) -> Self {
        Rgb(scale(self.0, v), scale(self.1, v), scale(self.2, v))
    }
}

/// a * b / 255
#[inline]
pub fn scale(a: u8, b: u8) -> u8 {
    ((a as u16 * b as u16 + 255) >> 8) as u8
}

#[inline]
pub fn sin8(x: u8) -> u8 {
    SIN8[x as usize]
}

/// Integer HSV -> RGB, all channels 0..=255, hue wraps.
pub fn hsv(h: u8, s: u8, v: u8) -> Rgb {
    if s == 0 {
        return Rgb(v, v, v);
    }
    let region = h / 43;
    let rem = ((h - region * 43) as u16 * 6) as u8;
    let p = scale(v, 255 - s);
    let q = scale(v, 255 - scale(s, rem));
    let t = scale(v, 255 - scale(s, 255 - rem));
    match region {
        0 => Rgb(v, t, p),
        1 => Rgb(q, v, p),
        2 => Rgb(p, v, t),
        3 => Rgb(p, q, v),
        4 => Rgb(t, p, v),
        _ => Rgb(v, p, q),
    }
}

/// xorshift32, good enough for sparkles and fire.
pub struct Rng(u32);

impl Rng {
    pub const fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x1234_5678 } else { seed })
    }

    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    pub fn byte(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }
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
pub const MASK: [u32; GRID_H] = grid_mask();

/// Is (row, col) one of the grid LEDs?
#[inline]
pub fn on_grid(r: i32, c: i32) -> bool {
    (0..GRID_H as i32).contains(&r) && (0..GRID_W as i32).contains(&c) && MASK[r as usize] >> c & 1 != 0
}

/// 5x7 text with its left edge at column `x` (may be negative or past the
/// grid: it is clipped), top row `row`.
pub fn draw_text(f: &mut Frame, s: &str, x: i32, row: usize, v: u8) {
    for (k, ch) in s.chars().enumerate() {
        let x0 = x + k as i32 * (font::WIDTH as i32 + 1);
        if x0 >= GRID_W as i32 || x0 + (font::WIDTH as i32) < 0 {
            continue;
        }
        let g = font::glyph(ch);
        for (dc, bits) in g.iter().enumerate() {
            for r in 0..font::HEIGHT {
                if bits >> r & 1 != 0 && x0 + dc as i32 >= 0 {
                    f.grid(row + r, (x0 + dc as i32) as usize, v);
                }
            }
        }
    }
}

/// Text that fits is centred; longer text scrolls left at 10 columns per
/// second, `t` in frames at `fps`.
pub fn draw_label(f: &mut Frame, s: &str, row: usize, t: u32, fps: u32, v: u8) {
    let w = font::text_width(s) as i32;
    if w <= GRID_W as i32 {
        draw_text(f, s, (GRID_W as i32 - w) / 2, row, v);
    } else {
        // copies follow each other every `period` columns: the one coming
        // in from the right, and the one before it, still going out on the left
        let period = w + 8;
        let x = GRID_W as i32 - (t * 10 / fps) as i32 % period;
        draw_text(f, s, x, row, v);
        draw_text(f, s, x - period, row, v);
    }
}

/// Number in 3x5 digits, centred, top row `row`.
pub fn draw_number(f: &mut Frame, n: u16, row: usize, v: u8) {
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
    let c0 = (GRID_W - (len * 4 - 1)) / 2;
    for k in 0..len {
        let rows = font::digit3x5(digits[len - 1 - k]);
        for (r, bits) in rows.iter().enumerate() {
            for c in 0..3 {
                if bits >> (2 - c) & 1 != 0 {
                    f.grid(row + r, c0 + k * 4 + c, v);
                }
            }
        }
    }
}
