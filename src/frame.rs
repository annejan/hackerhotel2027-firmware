//! Frame buffer and small integer colour/maths helpers.

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
