//! The two "badge" screens: your name, and Hacker Hotel 2027.
//!
//! The name is entered on the badge (EDIT NAME in the menu, see editor.rs)
//! and kept in flash (store.rs). Until then it is the build-time default:
//!   BADGE_NAME="ANNE JAN" cargo build --release
//! Letters, digits and ` !'*-.:?/#@` are drawn; lower case shows as upper.

use crate::anim::{phase, FPS};
use crate::frame::{draw_label, draw_text, hsv, sin8, Frame, Rgb, Rng};
use crate::font;
use crate::generated::{EDGE, GRID_W, RGB};

const DEFAULT: &str = match option_env!("BADGE_NAME") {
    Some(n) => n,
    None => "YOUR NAME",
};

/// Longest name, in characters.
pub const MAX: usize = 24;

/// A name: upper case, only characters the font has, no trailing spaces.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Name {
    buf: [u8; MAX],
    len: usize,
}

impl Name {
    pub fn new(s: &[u8]) -> Self {
        let mut n = Self { buf: [b' '; MAX], len: 0 };
        for (i, &c) in s.iter().take(MAX).enumerate() {
            let c = c.to_ascii_uppercase();
            n.buf[i] = if (b' '..=b'Z').contains(&c) { c } else { b' ' };
        }
        n.len = n.buf.iter().rposition(|&c| c != b' ').map_or(0, |i| i + 1);
        n
    }

    pub fn as_str(&self) -> &str {
        // only ASCII ' '..='Z' is ever stored
        crate::log::ascii(&self.buf[..self.len])
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

impl Default for Name {
    fn default() -> Self {
        Self::new(DEFAULT.as_bytes())
    }
}

/// Name tag: short names stand still, long ones scroll. A soft sheen moves
/// over the letters, the arms slowly cycle colour, the outline sparkles.
pub fn name(f: &mut Frame, t: u32, rng: &mut Rng, name: &Name) {
    draw_label(f, name.as_str(), 4, t, FPS, 255);
    // sheen: dim the letters except where a diagonal band passes
    let band = phase(t, 60) as i32;
    for r in 4..4 + font::HEIGHT {
        for c in 0..GRID_W {
            let s = crate::generated::GRID[r][c];
            if s != crate::generated::NONE && f.px[s as usize] > 0 {
                let d = ((c as i32 + r as i32) * 12 - band).rem_euclid(256);
                f.px[s as usize] = if d < 40 { 255 } else { 170 };
            }
        }
    }
    let w = phase(t, 20);
    for (i, l) in RGB.iter().enumerate() {
        f.rgb(i, hsv(w.wrapping_add(l.arm * 51), 200, 90 + sin8(w.wrapping_mul(3).wrapping_add(l.rank * 30)) / 3));
    }
    for &s in EDGE.iter() {
        if rng.byte() < 6 {
            f.set(s, 200);
        }
    }
}

/// Hacker Hotel 2027: "HH" over "2027" for 4 seconds, then the full name
/// scrolls by. The outline runs marquee lights like a hotel sign; the arms
/// glow warm gold.
pub fn hotel(f: &mut Frame, t: u32) {
    const SCROLL: &str = "HACKER HOTEL 2027";
    let logo = 4 * FPS;
    let scroll = (font::text_width(SCROLL) as u32 + GRID_W as u32) * FPS / 10;
    let k = t % (logo + scroll);
    if k < logo {
        draw_text(f, "HH", 3, 1, 255);
        let year = [2, 0, 2, 7];
        for (i, d) in year.iter().enumerate() {
            let rows = font::digit3x5(*d);
            for (r, bits) in rows.iter().enumerate() {
                for c in 0..3 {
                    if bits >> (2 - c) & 1 != 0 {
                        f.grid(9 + r, 1 + i * 4 + c, 200);
                    }
                }
            }
        }
    } else {
        let off = ((k - logo) * 10 / FPS) as i32;
        draw_text(f, SCROLL, GRID_W as i32 - off, 4, 255);
    }
    let step = (t * 8 / FPS) as usize; // marquee: 8 steps per second
    for (j, &s) in EDGE.iter().enumerate() {
        f.set(s, if (j + step) % 3 == 0 { 230 } else { 25 });
    }
    for (i, l) in RGB.iter().enumerate() {
        let flicker = sin8(phase(t, 90).wrapping_add(l.rank.wrapping_mul(37)).wrapping_add(l.arm.wrapping_mul(71)));
        f.rgb(i, Rgb(255, 140, 20).scaled(120 + flicker / 3));
    }
}
