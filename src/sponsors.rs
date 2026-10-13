//! The sponsor reel; same as `c/src/sponsors.c`: "THANKS TO", then every
//! sponsor's logo (when there is one) and name. It plays once, on the
//! first boot after the factory test (store.rs keeps that in flash);
//! `sponsors` on the UART console plays it again. Any tap skips it.
//!
//! A logo is GRID_H rows of GRID_W bits (bit c = column c), made from a
//! picture with `tools/sim/logo.py`.

use crate::anim::FPS;
use crate::font::text_width;
use crate::frame::{draw_text, hsv, Frame};
use crate::generated::{EDGE, GRID_H, GRID_W, RGB};

struct Sponsor {
    name: &'static str,
    logo: Option<&'static [u32; GRID_H]>,
}

// Logos: tools/sim/logo.py turns a picture into these rows.
const SPONSORS: [Sponsor; 3] = [
    Sponsor { name: "ALLNET.CHINA", logo: None },
    Sponsor { name: "BADGE.TEAM", logo: None },
    Sponsor { name: "NICOLAI ELECTRONICS", logo: None },
];

const SPEED: u32 = 20; // columns a second, twice the name tag's
const LOGO_FRAMES: u32 = 5 * FPS / 2;
const FADE: u32 = FPS / 2;

pub struct Reel {
    seg: usize,    // 0: "THANKS TO", then the sponsors
    on_logo: bool, // the logo, before the name
    t: u32,
    total: u32, // for the colours, over the whole reel
}

fn scroll_frames(s: &str) -> u32 {
    (GRID_W + text_width(s)) as u32 * FPS / SPEED
}

impl Reel {
    pub fn new() -> Self {
        Self { seg: 0, on_logo: false, t: 0, total: 0 }
    }

    fn begin(&mut self, seg: usize) {
        self.seg = seg;
        self.on_logo = seg >= 1 && SPONSORS.get(seg - 1).is_some_and(|s| s.logo.is_some());
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
        let text = sponsor.map_or("THANKS TO", |s| s.name);
        match sponsor.and_then(|s| s.logo) {
            Some(logo) if self.on_logo => {
                let v = if t < FADE { (t * 255 / FADE) as u8 } else { 255 };
                for (r, &bits) in logo.iter().enumerate() {
                    for c in 0..GRID_W {
                        if bits >> c & 1 != 0 {
                            f.grid(r, c, v);
                        }
                    }
                }
                if t + 1 >= LOGO_FRAMES {
                    self.on_logo = false;
                    self.t = 0;
                }
            }
            _ => {
                draw_text(f, text, GRID_W as i32 - (t * SPEED / FPS) as i32, 4, 255);
                if t + 1 >= scroll_frames(text) {
                    self.begin(self.seg + 1);
                }
            }
        }
        for (i, led) in RGB.iter().enumerate() {
            let h = (led.arm as u32 * 51 + led.rank as u32 * 8 + total * 2) as u8;
            f.rgb(i, hsv(h, 255, 160));
        }
        for (j, &s) in EDGE.iter().enumerate() {
            f.set(s, if (j as u32 + total / 2) % 8 == 0 { 160 } else { 10 });
        }
        true
    }
}
