//! The sponsor reel; same as `c/src/sponsors.c`: "THANKS TO", then the
//! sponsors' logos, with the arms in their colours. It plays once, on the
//! first boot after the factory test (store.rs keeps that in flash);
//! `sponsors` on the UART console plays it again. Any tap skips it.
//!
//! The logos are greyscale bitmaps in logos.rs, made from
//! `assets/sponsors` by `tools/sim/logo.py`. One that fits the grid stands
//! in the middle; a wider one scrolls by like text, and one taller than the
//! grid pans from its top to its bottom meanwhile (the ALLNET globe).

use crate::anim::FPS;
use crate::font::text_width;
use crate::frame::{draw_text, hsv, polar, scale, sin8, Frame, Rgb};
use crate::generated::{EDGE, GRID_H, GRID_W, RGB};
use crate::logos::{self, Logo};

#[derive(Clone, Copy)]
enum Arms {
    /// colours run along the arms
    Rainbow,
    /// a wave of the two colours runs out along the arms, which alternate
    Wave(Rgb, Rgb),
    /// a rainbow sweeps diagonally across the star
    Diagonal,
}

enum Show {
    Text(&'static str),
    Logo(&'static Logo),
}

// colours from the logos, brightened for LEDs
const ALLNET_BLUE: Rgb = Rgb(130, 178, 255);
const REEL: [(Show, Arms); 6] = [
    (Show::Text("THANKS TO"), Arms::Rainbow),
    (Show::Logo(&logos::ALLNET_GLOBE), Arms::Wave(ALLNET_BLUE, ALLNET_BLUE)),
    (Show::Logo(&logos::ALLNET), Arms::Wave(ALLNET_BLUE, ALLNET_BLUE)),
    (Show::Logo(&logos::BADGETEAM), Arms::Diagonal),
    (Show::Logo(&logos::NICOLAI), Arms::Wave(Rgb(240, 67, 0), Rgb(70, 70, 62))),
    (Show::Logo(&logos::DEFEEST), Arms::Wave(Rgb(255, 245, 109), Rgb(41, 145, 255))),
];

const SPEED: u32 = 20; // columns a second, twice the name tag's
const LOGO_FRAMES: u32 = 5 * FPS / 2; // a logo that fits stands this long
const FADE: u32 = FPS / 2;

pub struct Reel {
    item: usize,
    t: u32,     // frames into the item
    total: u32, // for the colours, over the whole reel
}

fn scroll_frames(w: usize) -> u32 {
    (GRID_W + w) as u32 * FPS / SPEED
}

fn fits(l: &Logo) -> bool {
    l.w as usize <= GRID_W && l.h as usize <= GRID_H
}

fn frames(show: &Show) -> u32 {
    match show {
        Show::Text(s) => scroll_frames(text_width(s)),
        Show::Logo(l) if fits(l) => LOGO_FRAMES,
        Show::Logo(l) => scroll_frames(l.w as usize),
    }
}

/// Greyscale logo with its top left corner at grid (y, x).
fn draw_logo(f: &mut Frame, l: &Logo, x: i32, y: i32, v: u8) {
    let (w, h) = (l.w as i32, l.h as i32);
    for row in 0..GRID_H as i32 {
        let lr = row - y;
        if !(0..h).contains(&lr) {
            continue;
        }
        for col in 0..GRID_W as i32 {
            let lc = col - x;
            if !(0..w).contains(&lc) {
                continue;
            }
            let i = (lr * w + lc) as usize;
            let level = l.data[i / 4] >> (2 * (i % 4)) & 3;
            if level != 0 {
                f.grid(row as usize, col as usize, scale(level * 85, v));
            }
        }
    }
}

/// One that fits stands in the middle, fading in; a wider one scrolls by,
/// and one taller than the grid pans from its top to its bottom meanwhile.
fn logo_frame(f: &mut Frame, l: &Logo, t: u32, n: u32) {
    let (w, h) = (l.w as i32, l.h as i32);
    let (gw, gh) = (GRID_W as i32, GRID_H as i32);
    if fits(l) {
        let v = if t < FADE { (t * 255 / FADE) as u8 } else { 255 };
        return draw_logo(f, l, (gw - w) / 2, (gh - h) / 2, v);
    }
    let y = if h <= gh { (gh - h) / 2 } else { -(((h - gh) as u32 * t / (n - 1)) as i32) };
    draw_logo(f, l, gw - (t * SPEED / FPS) as i32, y, 255);
}

fn arms(f: &mut Frame, style: Arms, total: u32) {
    for (i, led) in RGB.iter().enumerate() {
        let c = match style {
            Arms::Wave(a, b) => {
                let wave = sin8((led.rank as u32 * 36).wrapping_sub(total * 8) as u8) as u32;
                (if led.arm % 2 == 0 { a } else { b }).scaled((90 + wave * 165 / 255) as u8)
            }
            Arms::Diagonal => {
                // distance along the diagonal from bottom left to top right
                // (angle 32 of 256, clockwise from up), in 0.25 mm
                let (r, a) = polar(led.r);
                let along = r as i32 * (sin8(a.wrapping_add(32)) as i32 - 128) / 128;
                hsv(((along / 2) as u32).wrapping_sub(total * 4) as u8, 255, 200) // about one rainbow across the star
            }
            Arms::Rainbow => hsv((led.arm as u32 * 51 + led.rank as u32 * 8 + total * 2) as u8, 255, 160),
        };
        f.rgb(i, c);
    }
}

impl Reel {
    pub fn new() -> Self {
        Self { item: 0, t: 0, total: 0 }
    }

    /// False once the reel is over.
    pub fn render(&mut self, f: &mut Frame) -> bool {
        let Some((show, style)) = REEL.get(self.item) else { return false };
        let (t, total, n) = (self.t, self.total, frames(show));
        self.t += 1;
        self.total += 1;
        match show {
            Show::Text(s) => draw_text(f, s, GRID_W as i32 - (t * SPEED / FPS) as i32, 4, 255),
            Show::Logo(l) => logo_frame(f, l, t, n),
        }
        arms(f, *style, total);
        for (j, &s) in EDGE.iter().enumerate() {
            f.set(s, if (j as u32 + total / 2) % 8 == 0 { 160 } else { 10 });
        }
        if self.t >= n {
            self.item += 1;
            self.t = 0;
        }
        true
    }
}
