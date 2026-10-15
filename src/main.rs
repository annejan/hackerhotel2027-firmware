//! Hacker Hotel 2027 starfish badge, theoretical bare-metal firmware.
//!
//! Hardware (from ../hackerhotel2027-hardware):
//!   * WCH CH643Q, QingKe V4C RISC-V, 48 MHz, 62 KiB flash, 20 KiB RAM
//!   * 384 LED slots in a 24 x 16 matrix: PA0..PA23 drive anodes (PWM),
//!     PB0..PB15 sink cathodes (COM), scanned by the LEDPWM peripheral
//!       - 208 mono LEDs in a 17 x 16 grid in the centre
//!       -  71 mono LEDs along the outline
//!       -  35 RGB LEDs, 7 per arm (one PWM channel, three COMs each)
//!   * four push buttons ◀ B A ▶ (PC5, PB20, PC4, PB18, active low; see buttons.rs)
//!   * LiPo charger XT4054, CHRG status on PB22; VBUS sense on PC3
//!   * IR receiver (PB19) and IR LED (anode on VCC, PB21 cathode, low = on): ir.rs
//!   * UART header (USART3: PB16 TX, PB17 RX): uart.rs; SWD header (PC18 SWIO, PC19 SWCK)
//!
//! One superloop, like c/src/main.c: SysTick runs free on HCLK/8 and the
//! loop polls it (`due`) to scan the buttons (100 Hz), check CHRG (2 Hz),
//! decode IR (every 5 ms) and render frames (FPS). USB and the UART are
//! polled on every pass. Interrupts: the LED scanner (ledpwm.rs) and
//! infrared (ir.rs: TIM3 while sending, EXTI line 19 for the receiver).
#![no_std]
#![no_main]

mod anim;
mod app;
mod bootloader;
mod buttons;
mod challenge;
mod console;
mod ir;
mod irproto;
mod log;
mod selftest;
mod uart;
mod usb;
mod editor;
mod factory;
mod font;
mod frame;
mod game;
mod games;
mod generated;
mod ledpwm;
mod logos;
mod nametag;
mod proto;
mod sponsors;
mod store;

use core::ptr::read_volatile;

pub use ch32_metapac as pac;

use app::{App, Stage};
use frame::Frame;

/// HSI, no prescaler: what `clock_init` sets up.
pub const HCLK: u32 = 48_000_000;
/// SysTick counts HCLK/8.
pub const TICKS_PER_MS: u32 = HCLK / 8 / 1000;
const TICKS_PER_US: u32 = HCLK / 8 / 1_000_000;

const SYSTICK_CTLR: *mut u32 = 0xE000_F000 as *mut u32;
const SYSTICK_CNTL: *const u32 = 0xE000_F008 as *const u32;

/// SysTick's low word: wraps every 715 s at 6 MHz, `due` copes.
#[inline]
pub fn ticks() -> u32 {
    unsafe { read_volatile(SYSTICK_CNTL) }
}

pub fn ticks_to_us(t: u32) -> u32 {
    t / TICKS_PER_US
}

/// True once `ticks()` has reached `*next`, which then moves on by
/// `period`; late calls catch up one period at a time (as embassy's
/// Ticker did).
#[inline]
fn due(next: &mut u32, period: u32) -> bool {
    if (ticks().wrapping_sub(*next) as i32) < 0 {
        return false;
    }
    *next = next.wrapping_add(period);
    true
}

/// Things that happen to the badge, consumed by the render loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Button 0..=3 (◀ B A ▶) pressed; sent on press-down, for games
    Press(u8),
    /// Short press of button 0..=3, sent on release
    Tap(u8),
    /// Button 0..=3 held for `buttons::HOLD_MS`
    Hold(u8),
    /// USB power present and battery charging
    Charging(bool),
    /// ◀ + ▶ held for `buttons::BOOT_MS`: reboot into the USB boot ROM
    Bootloader,
    /// A + B held as long: the bring-up test (selftest.rs)
    Selftest,
}

/// PC3 sees VBUS (through R33 to GND): high while USB powers the badge.
fn vbus() -> bool {
    pac::GPIOC.indr().read().0 & 1 << 3 != 0
}

/// Events wait here for the next frame, as they waited in a channel
/// before; when it is full new ones are dropped.
struct Events {
    buf: [Event; 8],
    n: usize,
}

impl Events {
    fn push(&mut self, ev: Event) {
        if self.n < self.buf.len() {
            self.buf[self.n] = ev;
            self.n += 1;
        }
    }
}

/// XT4054 CHRG (PB22) is open-drain and pulled low while the battery
/// charges: input with pull-up.
const CHRG_PIN: u32 = 22;

fn charging() -> bool {
    pac::GPIOB.indr().read().0 & 1 << CHRG_PIN == 0
}

/// Pads that exist on the die but are not bonded out in the LQFP64 package
/// float and draw current unless pulled up. WCH's own SystemInit does this
/// (GPIO_IPD_Unused); ch32-hal does not. CH643Q: PC6..PC9, PC12, PC13, PC20.
/// PC0..PC2, PC14 and PC15 are bonded but not connected on this board (the
/// CC jumpers went in hardware commit 0986add), so pull those up too. PC3
/// (VBUS sense, 100k to GND) must stay a floating input.
fn pull_up_unused_pads() {
    use pac::{GPIOC, RCC};
    RCC.apb2pcenr().modify(|w| w.set_iopcen(true));
    // input with pull-up = mode nibble 0b1000, pull selected by OUTDR = 1
    GPIOC.cfglr().modify(|w| w.0 = (w.0 & 0x00FF_F000) | 0x8800_0888); // PC0..PC2, PC6, PC7
    // CFGHR is not readable on early silicon (WCH keeps a shadow copy), so
    // write it whole: PC8, PC9, PC12..PC15 pull-up, the rest floating input
    GPIOC.cfghr().write(|w| w.0 = 0x8888_4488);
    GPIOC.bshr().write(|w| w.0 = 0b111 | 1 << 6 | 1 << 7 | 1 << 8 | 1 << 9 | 0xF << 12);
    GPIOC.cfgxr().modify(|w| w.0 = (w.0 & !(0xF << 16)) | 0x8 << 16); // PC20
    GPIOC.bsxr().write(|w| w.0 = 1 << 4);
}

/// The CH643 runs on its 48 MHz HSI; ch32-hal's rcc init for this chip
/// (rcc/x0.rs) does the same: HSI on, two flash wait states, HCLK = SYSCLK.
fn clock_init() {
    use pac::{FLASH, RCC};
    RCC.ctlr().modify(|w| w.set_hsion(true));
    FLASH.actlr().modify(|w| w.set_latency(0b10));
    RCC.cfgr0().modify(|w| w.set_hpre(pac::rcc::vals::Hpre::DIV1));
    // ch32-hal's init also enabled and reset AFIO and enabled GPIOA..C
    RCC.apb2pcenr().modify(|w| {
        w.set_afioen(true);
        w.set_iopaen(true);
        w.set_iopben(true);
        w.set_iopcen(true);
    });
}

#[qingke_rt::entry]
fn main() -> ! {
    clock_init();
    pull_up_unused_pads();
    // SysTick: reset the count, enable, HCLK/8, count up (as c/src/main.c)
    unsafe { core::ptr::write_volatile(SYSTICK_CTLR, 1 << 5 | 1 << 0) };

    // PB22 (CHRG) input with pull-up: CFGXR nibble 6 = 0b1000, pull-up by BSXR
    pac::GPIOB.cfgxr().modify(|w| w.0 = (w.0 & !(0xF << 24)) | 0x8 << 24);
    pac::GPIOB.bsxr().write(|w| w.0 = 1 << (CHRG_PIN - 16));
    let mut buttons = buttons::Buttons::new();

    // LEDPWM owns PA0..PA23 and PB0..PB15; nothing else may touch those pins.
    let mut display = ledpwm::LedPwm::new();
    let name = store::load().unwrap_or_default();
    log::text("\r\nhh27 starfish badge, rust firmware. Name: ");
    log::line(name.as_str());
    log::line("Type help.");
    let mut stage = store::stage();
    let mut app = first_app(name, stage, store::progress());
    let mut ir = ir::Ir::new();
    let mut uart = uart::Uart::new();
    let mut usb = usb::Usb::new();
    let mut frame = Frame::new();
    let mut events = Events { buf: [Event::Selftest; 8], n: 0 };
    let mut last_charging = None;
    // at most one name save per SAVE_FRAMES: a script renaming the badge
    // over USB must not wear out the page; the last name always gets saved
    let mut pending = None;
    let mut since_save = SAVE_FRAMES;

    const FRAME_TICKS: u32 = HCLK / 8 / anim::FPS;
    let now = ticks();
    let (mut next_frame, mut next_buttons, mut next_chrg, mut next_ir) = (now, now, now, now);
    loop {
        usb.poll(|packet, reply| proto::handle(&mut app, packet, reply));
        uart.poll(|line| console::line(&mut app, line));
        if due(&mut next_ir, 5 * TICKS_PER_MS) {
            ir.poll(|ty, data| app.ir_received(ty, data));
            ir.set_loopback(app.ir_loopback());
            if !ir.busy() {
                if let Some((ty, name)) = app.ir_take() {
                    ir.send(ty, name.bytes());
                }
            }
        }
        if due(&mut next_buttons, buttons::POLL_MS * TICKS_PER_MS) {
            buttons.poll(|ev| events.push(ev));
        }
        if due(&mut next_chrg, 500 * TICKS_PER_MS) {
            let c = charging();
            if last_charging != Some(c) {
                events.push(Event::Charging(c));
                last_charging = Some(c);
            }
        }
        if !due(&mut next_frame, FRAME_TICKS) {
            continue;
        }

        for &ev in &events.buf[..events.n] {
            if ev == Event::Bootloader {
                // show it for a second, so a held chord is visibly accepted
                let level = app.brightness().max(32);
                for t in 0..anim::FPS {
                    frame.clear();
                    anim::usb_boot(&mut frame, t);
                    display.show(&frame, level);
                    while !due(&mut next_frame, FRAME_TICKS) {}
                }
                bootloader::reboot_to_boot_rom();
            }
            app.handle(ev);
        }
        events.n = 0;
        if let Some(name) = app.saved_name() {
            pending = Some(name);
        }
        since_save = since_save.saturating_add(1);
        if let Some(name) = pending.take_if(|_| since_save >= SAVE_FRAMES) {
            since_save = 0;
            store::save(&name); // a page erase and program: a few ms
            log::text("name saved: ");
            log::line(name.as_str());
        }
        if let Some(p) = app.progress() {
            store::set_progress(p);
            log::text("challenges: ");
            log::num(app.challenges().total() as u32);
            log::line(" of 25");
        }
        if let Some(next) = app.stage() {
            if next > stage {
                // only forward: a re-run test does not bring the reel back
                store::set_stage(next);
                stage = next;
            }
        }
        frame.clear();
        app.hw = selftest::Hw { rows: ledpwm::rows(), vbus: vbus() };
        app.render(&mut frame, buttons.held());
        display.show(&frame, app.brightness());
    }
}

/// Five seconds between name saves.
const SAVE_FRAMES: u16 = 5 * anim::FPS as u16;

/// The app at power on: a new badge starts with the factory test, one
/// that just passed it with the sponsor reel.
fn first_app(name: nametag::Name, stage: Stage, progress: u16) -> App {
    let mut app = App::new(name);
    app.load_progress(progress);
    match stage {
        Stage::New => app.factory(),
        Stage::Tested => app.sponsors(),
        Stage::Done => {}
    }
    app
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
