//! The sponsor reel; same as `c/src/sponsors.c`: "THANKS TO", then every
//! sponsor's logo and, where the logo does not say it, the name. It plays
//! once, on the first boot after the factory test (store.rs keeps that in
//! flash); `sponsors` on the UART console plays it again. Any tap skips it.
//!
//! The logos are greyscale bitmaps in logos.rs, made from
//! `assets/sponsors` by `tools/sim/logo.py`. One that fits the grid (17
//! columns) stands in the middle; a wider one scrolls by like text.

use crate::anim::FPS;
use crate::font::text_width;
use crate::frame::{draw_text, hsv, scale, sin8, Frame, Rgb};
use crate::generated::{EDGE, GRID_H, GRID_W, RGB};
use crate::logos;

struct Sponsor {
    text: Option<&'static str>, // scrolls after the logo; None when the logo says it
    logo: &'static [u32],       // one u32 per column, 2 bits a row; empty for none
    arm: [Rgb; 2],              // the arms alternate between these
}

// colours from the logos, brightened for LEDs
const SPONSORS: [Sponsor; 3] = [
    Sponsor { text: Some("ALLNET.CHINA"), logo: &logos::ALLNET, arm: [Rgb(130, 178, 255), Rgb(130, 178, 255)] },
    Sponsor { text: None, logo: &logos::BADGETEAM, arm: [Rgb(255, 255, 255), Rgb(255, 255, 255)] },
    Sponsor { text: None, logo: &logos::NICOLAI, arm: [Rgb(240, 67, 0), Rgb(70, 70, 62)] },
];

const SPEED: u32 = 20; // columns a second, twice the name tag's
const LOGO_FRAMES: u32 = 5 * FPS / 2; // a logo that fits stands this long
const FADE: u32 = FPS / 2;

pub struct Reel {
    seg: usize,    // 0: "THANKS TO", then the sponsors
    on_logo: bool, // the logo, before the text
    t: u32,
    total: u32, // for the colours, over the whole reel
}

fn scroll_frames(w: usize) -> u32 {
    (GRID_W + w) as u32 * FPS / SPEED
}

/// Greyscale logo, column 0 at grid column `x`.
fn draw_logo(f: &mut Frame, logo: &[u32], x: i32, v: u8) {
    for (c, &col) in logo.iter().enumerate() {
        let gx = x + c as i32;
        if !(0..GRID_W as i32).contains(&gx) {
            continue;
        }
        for row in 0..GRID_H {
            let level = (col >> (2 * row) & 3) as u8;
            if level != 0 {
                f.grid(row, gx as usize, scale(level * 85, v));
            }
        }
    }
}

/// The logo, standing in the middle or scrolling by; true when it is over.
fn logo_frame(f: &mut Frame, logo: &[u32], t: u32) -> bool {
    if logo.len() <= GRID_W {
        let v = if t < FADE { (t * 255 / FADE) as u8 } else { 255 };
        draw_logo(f, logo, (GRID_W - logo.len()) as i32 / 2, v);
        return t + 1 >= LOGO_FRAMES;
    }
    draw_logo(f, logo, GRID_W as i32 - (t * SPEED / FPS) as i32, 255);
    t + 1 >= scroll_frames(logo.len())
}

impl Reel {
    pub fn new() -> Self {
        Self { seg: 0, on_logo: false, t: 0, total: 0 }
    }

    fn begin(&mut self, seg: usize) {
        self.seg = seg;
        self.on_logo = seg >= 1 && SPONSORS.get(seg - 1).is_some_and(|s| !s.logo.is_empty());
        self.t = 0;
    }

    /// False once the reel is over.
    pub fn render(&mut self, f: &mut Frame) -> bool {
        if self.seg > SPONSORS.len() {
            return false;
        }
        let (t, total) = (self.t, self.total);
        self.t += 1;
        self.total += 1;
        let sponsor = self.seg.checked_sub(1).map(|i| &SPONSORS[i]);
        match sponsor {
            Some(s) if self.on_logo => {
                if logo_frame(f, s.logo, t) {
                    self.on_logo = false;
                    self.t = 0;
                    if s.text.is_none() {
                        self.begin(self.seg + 1);
                    }
                }
            }
            _ => {
                let text = sponsor.map_or(Some("THANKS TO"), |s| s.text).unwrap_or("");
                draw_text(f, text, GRID_W as i32 - (t * SPEED / FPS) as i32, 4, 255);
                if t + 1 >= scroll_frames(text_width(text)) {
                    self.begin(self.seg + 1);
                }
            }
        }
        for (i, led) in RGB.iter().enumerate() {
            let c = match sponsor {
                // a wave of the sponsor's colours runs out along the arms
                Some(s) => {
                    let wave = sin8((led.rank as u32 * 36).wrapping_sub(total * 8) as u8) as u32;
                    s.arm[led.arm as usize % 2].scaled((90 + wave * 165 / 255) as u8)
                }
                None => hsv((led.arm as u32 * 51 + led.rank as u32 * 8 + total * 2) as u8, 255, 160),
            };
            f.rgb(i, c);
        }
        for (j, &s) in EDGE.iter().enumerate() {
            f.set(s, if (j as u32 + total / 2) % 8 == 0 { 160 } else { 10 });
        }
        true
    }
}
