//! Factory test: the first thing a freshly flashed badge does, for the
//! volunteers at the flashing table; same as `c/src/factory.c`. Short and
//! pass / fail, for people without a serial terminal; the full bring-up
//! test (selftest.rs) is for engineers. main.rs starts it until it has
//! passed once (store.rs keeps that in flash); `factory` on the UART
//! console starts it too.
//!
//!   LED  every LED on: white, then the arms red, green, blue. B = all lit,
//!        A = something dark or the wrong colour
//!   ROW  one row top to bottom, then one column left to right (a COM line,
//!        then a PWM channel): exactly one moving line or set of dots,
//!        never two. B = good, A = fault
//!   KEY  press ◀, B, A and ▶ once each (automatic; 20 s)
//!   USB  USB power seen on PC3 for a second (automatic; 10 s)
//!   IR   sends test packets until one is heard: its own echo, or another
//!        badge's test packets (automatic; 10 s)
//!
//! A failed step shows its name in red; B tries that step again. A + B
//! held for 3 s goes to the bring-up test instead. Every step is logged on
//! the UART as well.

use crate::anim::FPS;
use crate::font::text_width;
use crate::frame::{draw_number, draw_text, Frame, Rgb};
use crate::generated::{COMS, EDGE, GRID, GRID_W, PWM_CHANNELS, RGB};
use crate::irproto;
use crate::log;
use crate::selftest::Hw;
use crate::Event;

const NAMES: [&str; 5] = ["LED", "ROW", "KEY", "USB", "IR"];
const HINTS: [&str; 5] = [
    "all LEDs lit, arms red / green / blue? B = yes, A = no",
    "one line or one set of dots at a time, never two? B = yes, A = no",
    "press < B A > once each",
    "plug in USB",
    "listening for its own echo or another badge",
];
const LED: u8 = 0;
const ROW: u8 = 1;
const KEY: u8 = 2;
const USB: u8 = 3;
const IR: u8 = 4;
const BUTTON: [&str; 4] = ["<", "B", "A", ">"];

const LABEL: u32 = FPS; // the step's name shows this long first
const ROW_FRAMES: u32 = 4; // per line in ROW
const KEY_TIMEOUT: u32 = 20 * FPS;
const USB_TIMEOUT: u32 = 10 * FPS;
const USB_STEADY: u32 = FPS;
const IR_EVERY: u32 = FPS;
const IR_TIMEOUT: u32 = 10 * FPS;
const PASS_FRAMES: u32 = 3 * FPS;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Running,
    Pass,
    Fail,
}

/// What a frame of the factory test means for the app.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Busy,
    /// Once, the frame it passes: keep that in flash.
    Passed,
    /// The pass screen is over, back to the badge.
    Done,
}

pub struct Factory {
    step: u8,
    state: State,
    t: u32,     // frames since the step (or the pass screen) started
    keys: u8,   // buttons seen in KEY, bit n = button n
    usb_t: u32, // frames USB has been there without a break
    ir_sent: u16,
    ir_heard: u16,
}

impl Factory {
    pub fn new() -> Self {
        log::line("factory test: 5 steps, see docs/USAGE.md");
        let mut f = Self { step: LED, state: State::Running, t: 0, keys: 0, usb_t: 0, ir_sent: 0, ir_heard: 0 };
        f.start(LED);
        f
    }

    fn start(&mut self, step: u8) {
        *self = Self { step, state: State::Running, t: 0, keys: 0, usb_t: 0, ir_sent: 0, ir_heard: 0 };
        log::text("factory test ");
        log::num(step as u32 + 1);
        log::text("/");
        log::num(NAMES.len() as u32);
        log::text(" ");
        log::text(NAMES[step as usize]);
        log::text(": ");
        log::line(HINTS[step as usize]);
    }

    fn next(&mut self) {
        if (self.step as usize) < NAMES.len() - 1 {
            return self.start(self.step + 1);
        }
        self.state = State::Pass;
        self.t = 0;
        log::line("factory test: PASS");
    }

    fn fail(&mut self, why: &str) {
        self.state = State::Fail;
        self.t = 0;
        log::text("factory test: FAIL at ");
        log::text(NAMES[self.step as usize]);
        log::text(": ");
        log::line(why);
        log::line("B tries the step again");
    }

    /// The IR step wants to hear its own packets.
    pub fn loopback(&self) -> bool {
        self.state == State::Running && self.step == IR
    }

    pub fn handle(&mut self, ev: Event) {
        if self.state == State::Fail {
            if ev == Event::Tap(1) {
                self.start(self.step);
            }
            return;
        }
        if self.state != State::Running || self.t < LABEL {
            return;
        }
        match (self.step, ev) {
            (KEY, Event::Press(b)) if b < 4 => {
                self.keys |= 1 << b;
                log::text("key ");
                log::line(BUTTON[b as usize]);
                if self.keys == 0xF {
                    self.next();
                }
            }
            (LED | ROW, Event::Tap(1)) => self.next(),
            (LED | ROW, Event::Tap(2)) => self.fail("marked bad (A)"),
            _ => {}
        }
    }

    /// A packet of type TEST was heard.
    pub fn ir_heard(&mut self) {
        if self.state == State::Running && self.step == IR {
            self.ir_heard += 1;
            log::line("ir: test packet heard");
        }
    }

    /// One frame. `ir_out` takes a packet to send (main.rs passes it on).
    pub fn render(&mut self, f: &mut Frame, hw: Hw, charging: bool, ir_out: &mut Option<irproto::Packet>) -> Status {
        let now = self.t;
        self.t += 1;
        match self.state {
            State::Pass => {
                arms(f, Rgb(0, 255, 40));
                for &s in EDGE.iter() {
                    f.set(s, 200);
                }
                center(f, "OK", 4, 255);
                return if now == 0 {
                    Status::Passed
                } else if now >= PASS_FRAMES {
                    Status::Done
                } else {
                    Status::Busy
                };
            }
            State::Fail => {
                arms(f, if now / (FPS / 2) % 2 != 0 { Rgb(255, 0, 0) } else { Rgb(60, 0, 0) });
                center(f, NAMES[self.step as usize], 2, 255);
                center(f, "B", 9, 60);
                return Status::Busy;
            }
            State::Running => {}
        }
        if now < LABEL {
            center(f, NAMES[self.step as usize], 4, 255);
            self.progress(f);
            return Status::Busy;
        }
        let t = now - LABEL;
        match self.step {
            LED => {
                const COLOUR: [Rgb; 3] = [Rgb(255, 0, 0), Rgb(0, 255, 0), Rgb(0, 0, 255)];
                let p = t % (FPS * 9 / 2); // 1.5 s white, then 1 s per colour
                f.px.fill(255);
                if p >= FPS * 3 / 2 {
                    arms(f, COLOUR[((p - FPS * 3 / 2) / FPS) as usize]);
                }
            }
            ROW => {
                // in the order you see them, not wiring order (row 7 is
                // PB15, column 7 is PA9): the COM line of grid row k top to
                // bottom, then the PWM channel of grid column k left to
                // right, then PA17..PA23, which only drive the outline and
                // the arms. Column 8 has a LED in every row, row 7 in every
                // column.
                let k = (t / ROW_FRAMES) as usize % (COMS + PWM_CHANNELS);
                if k < COMS {
                    let com = GRID[k][8] as usize / PWM_CHANNELS;
                    f.px[com * PWM_CHANNELS..][..PWM_CHANNELS].fill(255);
                } else {
                    let k = k - COMS;
                    let pwm = if k < GRID_W { GRID[7][k] as usize % PWM_CHANNELS } else { k };
                    for c in 0..COMS {
                        f.px[c * PWM_CHANNELS + pwm] = 255;
                    }
                }
            }
            KEY => {
                for (i, b) in BUTTON.iter().enumerate() {
                    let v = if self.keys >> i & 1 != 0 { 255 } else { 25 };
                    draw_text(f, b, if i % 2 != 0 { 10 } else { 2 }, if i < 2 { 2 } else { 9 }, v);
                }
                self.progress(f);
                if t >= KEY_TIMEOUT {
                    let mut missing = *b"missing: < B A >";
                    for i in 0..4 {
                        if self.keys >> i & 1 != 0 {
                            missing[9 + i * 2] = b'.';
                        }
                    }
                    self.fail(crate::log::ascii(&missing));
                }
            }
            USB => {
                self.usb_t = if hw.vbus { self.usb_t + 1 } else { 0 };
                center(f, "USB", 2, if hw.vbus || now / (FPS / 2) % 2 != 0 { 255 } else { 30 });
                if charging {
                    center(f, "CHG", 9, 120);
                }
                self.progress(f);
                if self.usb_t >= USB_STEADY {
                    self.next();
                } else if t >= USB_TIMEOUT {
                    self.fail("no USB power on PC3");
                }
            }
            _ => {
                if t % IR_EVERY == 0 && ir_out.is_none() {
                    *ir_out = Some(irproto::Packet::new(irproto::TEST, b"TEST"));
                    self.ir_sent += 1;
                }
                draw_number(f, self.ir_heard, 2, 255);
                draw_number(f, self.ir_sent, 9, 80);
                self.progress(f);
                if self.ir_heard > 0 {
                    self.next();
                } else if t >= IR_TIMEOUT {
                    self.fail("nothing heard");
                }
            }
        }
        Status::Busy
    }

    /// The outline fills up as the steps go by.
    fn progress(&self, f: &mut Frame) {
        let n = EDGE.len();
        for (j, &s) in EDGE.iter().enumerate() {
            if j * NAMES.len() < (self.step as usize + 1) * n {
                f.set(s, 60);
            }
        }
    }
}

fn center(f: &mut Frame, s: &str, row: usize, v: u8) {
    draw_text(f, s, (GRID_W - text_width(s)) as i32 / 2, row, v);
}

fn arms(f: &mut Frame, c: Rgb) {
    for i in 0..RGB.len() {
        f.rgb(i, c);
    }
}
