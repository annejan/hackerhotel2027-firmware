//! Bring-up tests for a first board; same as `c/src/selftest.c`. Start:
//! hold A + B for 3 s, or `test` on the UART console. ◀ / ▶ pick the test,
//! hold A leaves. Every step is logged on the UART (log.rs).
//!
//!   COM  all 24 PWM channels on, one COM line at a time (2 a second)
//!   PWM  one PWM channel on all 16 COM lines at a time
//!   LED  one LED at a time, with its D number; B pauses, tap A steps
//!   ALL  every LED on, at the current brightness (current draw)
//!   GRY  grey ramp on the grid, red / green / blue / white / orange arms
//!   KEY  the buttons held, as letters; every button event is logged anyway
//!   SCN  LED rows per second; 2930 means the LED clock is 48 MHz HCLK
//!   IR   sends a test packet every 2 s and counts the ones it hears back
//!   PWR  USB power (PC3) and charging (CHRG)

use crate::anim::FPS;
use crate::font::text_width;
use crate::frame::{draw_number, draw_text, Frame, Rgb};
use crate::generated::{COMS, EDGE, GRID_H, GRID_W, PWM_CHANNELS, RGB, SLOTS, SLOT_REF};
use crate::irproto;
use crate::log;
use crate::nametag::Name;
use crate::Event;

const NAMES: [&str; 9] = ["COM", "PWM", "LED", "ALL", "GRY", "KEY", "SCN", "IR", "PWR"];
const COM: u8 = 0;
const PWM: u8 = 1;
const LED: u8 = 2;
const ALL: u8 = 3;
const GREY: u8 = 4;
const KEY: u8 = 5;
const SCAN: u8 = 6;
const IR: u8 = 7;
const POWER: u8 = 8;
const BUTTON: [&str; 4] = ["<", "B", "A", ">"]; // ◀ B A ▶ in the log

const LABEL_FRAMES: u32 = FPS; // the test's name shows this long first
const STEP_FRAMES: u32 = FPS / 2;
const LED_FRAMES: u32 = FPS / 4;
const IR_EVERY: u32 = 2 * FPS;

/// What the test reads from the hardware; main.rs fills it in each frame.
#[derive(Clone, Copy, Default)]
pub struct Hw {
    /// LED rows shown since power on (ledpwm.rs)
    pub rows: u32,
    /// USB power present (PC3)
    pub vbus: bool,
}

pub struct Selftest {
    test: u8,
    t: u32,
    step: usize,
    shown: Option<usize>,
    paused: bool,
    rows0: u32,
    rows_t0: u32,
    rate: u32,
    ir_sent: u16,
    ir_heard: u16,
    power: Option<(bool, bool)>,
}

impl Selftest {
    pub fn new(hw: Hw) -> Self {
        log::line("bring-up test: < > pick a test, hold A to leave");
        let mut s = Self {
            test: 0,
            t: 0,
            step: 0,
            shown: None,
            paused: false,
            rows0: 0,
            rows_t0: 0,
            rate: 0,
            ir_sent: 0,
            ir_heard: 0,
            power: None,
        };
        s.start(COM, hw);
        s
    }

    fn start(&mut self, test: u8, hw: Hw) {
        self.test = test;
        self.rows0 = hw.rows;
        self.t = 0;
        self.step = 0;
        self.shown = None;
        self.paused = false;
        self.rows_t0 = 0;
        self.rate = 0;
        self.ir_sent = 0;
        self.ir_heard = 0;
        self.power = None;
        log::text("test ");
        log::num(test as u32 + 1);
        log::text("/");
        log::num(NAMES.len() as u32);
        log::text(": ");
        log::line(NAMES[test as usize]);
    }

    /// The IR test wants to hear its own packets.
    pub fn loopback(&self) -> bool {
        self.test == IR
    }

    /// False once the test mode is left (hold A).
    pub fn handle(&mut self, ev: Event, hw: Hw) -> bool {
        let (kind, b) = match ev {
            Event::Press(b) => ("press", b),
            Event::Tap(b) => ("tap", b),
            Event::Hold(b) => ("hold", b),
            _ => return true,
        };
        if let Some(name) = BUTTON.get(b as usize) {
            log::text("key ");
            log::text(kind);
            log::text(" ");
            log::line(name);
        }
        let n = NAMES.len() as u8;
        match ev {
            Event::Hold(2) => {
                log::line("bring-up test: done");
                return false;
            }
            Event::Tap(0) => self.start((self.test + n - 1) % n, hw),
            Event::Tap(3) => self.start((self.test + 1) % n, hw),
            Event::Tap(1) if self.test == LED => self.paused = !self.paused,
            Event::Tap(2) if self.test == LED && self.paused => self.step += 1,
            _ => {}
        }
        true
    }

    /// A packet of type TEST came back.
    pub fn ir_heard(&mut self) {
        self.ir_heard += 1;
        log::text("ir: heard ");
        log::num(self.ir_heard as u32);
        log::text(" of ");
        log::num(self.ir_sent as u32);
        log::line(" sent");
    }

    fn walk(&mut self, every: u32, count: usize) {
        if !self.paused && self.t % every == 0 && self.t != 0 {
            self.step += 1;
        }
        self.step %= count;
    }

    fn new_step(&mut self) -> bool {
        if self.shown == Some(self.step) {
            return false;
        }
        self.shown = Some(self.step);
        true
    }

    fn log_slot(slot: usize) {
        let r = SLOT_REF[slot];
        log::text("slot ");
        log::num(slot as u32);
        log::text(": PB");
        log::num((slot / PWM_CHANNELS) as u32);
        log::text(" PA");
        log::num((slot % PWM_CHANNELS) as u32);
        if r == 0 {
            return log::line(" no LED");
        }
        log::text(" D");
        log::num((r & 0x0FFF) as u32);
        log::line(["", " red", " green", " blue"][(r >> 12 & 3) as usize]);
    }

    /// One frame. `ir_out` takes a packet to send (main.rs passes it on).
    pub fn render(&mut self, f: &mut Frame, held: u8, hw: Hw, level: usize, charging: bool, ir_out: &mut Option<(u8, Name)>) {
        let now = self.t;
        self.t += 1;
        let name = NAMES[self.test as usize];
        if now < LABEL_FRAMES && self.test != KEY {
            return draw_text(f, name, (GRID_W - text_width(name)) as i32 / 2, 4, 255);
        }
        match self.test {
            COM => {
                self.walk(STEP_FRAMES, COMS);
                for p in 0..PWM_CHANNELS {
                    f.px[self.step * PWM_CHANNELS + p] = 255;
                }
                if self.new_step() {
                    log::text("COM PB");
                    log::num(self.step as u32);
                    log::line(" on");
                }
            }
            PWM => {
                self.walk(STEP_FRAMES, PWM_CHANNELS);
                for c in 0..COMS {
                    f.px[c * PWM_CHANNELS + self.step] = 255;
                }
                if self.new_step() {
                    log::text("PWM PA");
                    log::num(self.step as u32);
                    log::line(" on");
                }
            }
            LED => {
                self.walk(LED_FRAMES, SLOTS);
                f.px[self.step] = 255;
                if self.new_step() {
                    Self::log_slot(self.step);
                }
            }
            ALL => {
                f.px.fill(255);
                if now == LABEL_FRAMES {
                    log::text("all LEDs on, brightness level ");
                    log::num(level as u32);
                    log::line(" (0 night .. 4 full)");
                }
            }
            GREY => {
                for r in 0..GRID_H {
                    for c in 0..GRID_W {
                        f.grid(r, c, (c * 255 / (GRID_W - 1)) as u8);
                    }
                }
                for (j, &s) in EDGE.iter().enumerate() {
                    f.set(s, (j * 255 / (EDGE.len() - 1)) as u8);
                }
                const ARM: [Rgb; 5] = [Rgb(255, 0, 0), Rgb(0, 255, 0), Rgb(0, 0, 255), Rgb(255, 255, 255), Rgb(255, 160, 0)];
                for (i, led) in RGB.iter().enumerate() {
                    f.rgb(i, ARM[led.arm as usize].scaled(255 - led.rank * 30));
                }
                if now == LABEL_FRAMES {
                    log::line("grid: grey ramp left to right; arms red, green, blue, white, orange");
                }
            }
            KEY => {
                for (i, glyph) in ["L", "B", "A", "R"].iter().enumerate() {
                    if held >> i & 1 != 0 {
                        draw_text(f, glyph, 1 + i as i32 * 4, 4, 255);
                    }
                }
            }
            SCAN => {
                let rows = hw.rows.wrapping_sub(self.rows0);
                if now - self.rows_t0 >= FPS {
                    self.rate = rows * FPS / (now - self.rows_t0);
                    self.rows0 = self.rows0.wrapping_add(rows);
                    self.rows_t0 = now;
                    log::text("scan: ");
                    log::num(self.rate);
                    log::line(" rows/s (2930 = 48 MHz LED clock, 183 Hz frames)");
                }
                draw_number(f, self.rate.min(65535) as u16, 5, 255);
            }
            IR => {
                if now % IR_EVERY == 0 && ir_out.is_none() {
                    *ir_out = Some((irproto::TEST, Name::new(b"TEST")));
                    self.ir_sent += 1;
                    log::text("ir: test packet ");
                    log::num(self.ir_sent as u32);
                    log::line(" sent");
                }
                draw_number(f, self.ir_heard, 2, 255);
                draw_number(f, self.ir_sent, 9, 80);
            }
            POWER => {
                let label = if hw.vbus { "USB" } else { "BAT" };
                draw_text(f, label, (GRID_W - text_width(label)) as i32 / 2, 2, 255);
                if charging {
                    draw_text(f, "CHG", (GRID_W - text_width("CHG")) as i32 / 2, 9, 120);
                }
                if self.power != Some((hw.vbus, charging)) {
                    self.power = Some((hw.vbus, charging));
                    log::text("power: ");
                    log::text(if hw.vbus { "USB (PC3 high)" } else { "battery (PC3 low)" });
                    log::line(if charging { ", charging (CHRG low)" } else { ", not charging" });
                }
            }
            _ => {}
        }
    }
}
