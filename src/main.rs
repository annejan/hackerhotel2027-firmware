//! Hacker Hotel 2027 starfish badge, theoretical Embassy firmware.
//!
//! Hardware (from ../hackerhotel2027-hardware):
//!   * WCH CH643Q, QingKe V4C RISC-V, 48 MHz, 62 KiB flash, 20 KiB RAM
//!   * 384 LED slots in a 24 x 16 matrix: PA0..PA23 drive anodes (PWM),
//!     PB0..PB15 sink cathodes (COM), scanned by the LEDPWM peripheral
//!       - 208 mono LEDs in a 17 x 16 grid in the centre
//!       -  71 mono LEDs along the outline
//!       -  35 RGB LEDs, 7 per arm (one PWM channel, three COMs each)
//!   * touch pads T1..T4 on PC0..PC3 (TouchKey channels 10..13)
//!   * LiPo charger XT4054, CHRG status on PB22
//!
//! Tasks:
//!   main loop      renders the current animation at FPS and hands frames to the display
//!   touch::scan    polls the touch pads, sends Press/Tap/Hold/Bootloader events
//!   charge_monitor watches CHRG, sends Charging events
#![no_std]
#![no_main]

mod anim;
mod app;
mod bootloader;
mod editor;
mod font;
mod frame;
mod game;
mod games;
mod generated;
mod ledpwm;
mod nametag;
mod store;
mod touch;

use ch32_hal as hal;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker, Timer};
use hal::gpio::{Input, Pull};

use app::App;
use frame::Frame;

/// Things that happen to the badge, consumed by the render loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Pad 0..=3 (T1..T4) touched; sent on touch-down, for games
    Press(u8),
    /// Short touch on pad 0..=3, sent on release
    Tap(u8),
    /// Touch held for `touch::HOLD_MS` on pad 0..=3
    Hold(u8),
    /// USB power present and battery charging
    Charging(bool),
    /// T1 + T4 held for `touch::BOOT_MS`: reboot into the USB boot ROM
    Bootloader,
}

pub static EVENTS: Channel<CriticalSectionRawMutex, Event, 8> = Channel::new();

/// XT4054 CHRG is open-drain and pulled low while the battery charges.
#[embassy_executor::task]
async fn charge_monitor(chrg: Input<'static>) {
    let mut last = None;
    loop {
        let charging = chrg.is_low();
        if last != Some(charging) {
            EVENTS.send(Event::Charging(charging)).await;
            last = Some(charging);
        }
        Timer::after_millis(500).await;
    }
}

/// Pads that exist on the die but are not bonded out in the LQFP64 package
/// float and draw current unless pulled up. WCH's own SystemInit does this
/// (GPIO_IPD_Unused); ch32-hal does not. CH643Q: PC6..PC9, PC12, PC13, PC20.
fn pull_up_unbonded_pads() {
    use hal::pac::{GPIOC, RCC};
    RCC.apb2pcenr().modify(|w| w.set_iopcen(true));
    // input with pull-up = mode nibble 0b1000, pull selected by OUTDR = 1
    GPIOC.cfglr().modify(|w| w.0 = (w.0 & 0x00FF_FFFF) | 0x8800_0000); // PC6, PC7
    // CFGHR is not readable on early silicon (WCH keeps a shadow copy), so
    // write it whole: PC8, PC9, PC12, PC13 pull-up, the rest floating input
    GPIOC.cfghr().write(|w| w.0 = 0x4488_4488);
    GPIOC.bshr().write(|w| w.0 = 1 << 6 | 1 << 7 | 1 << 8 | 1 << 9 | 1 << 12 | 1 << 13);
    GPIOC.cfgxr().modify(|w| w.0 = (w.0 & !(0xF << 16)) | 0x8 << 16); // PC20
    GPIOC.bsxr().write(|w| w.0 = 1 << 4);
}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let mut config = hal::Config::default();
    config.rcc = hal::rcc::Config::SYSCLK_FREQ_48MHZ_HSI;
    let p = hal::init(config);
    pull_up_unbonded_pads();

    spawner.spawn(charge_monitor(Input::new(p.PB22, Pull::Up)).unwrap());
    spawner.spawn(touch::scan(p.ADC1, p.PC0, p.PC1, p.PC2, p.PC3).unwrap());

    // LEDPWM owns PA0..PA23 and PB0..PB15; nothing else may touch those pins.
    let mut display = ledpwm::LedPwm::new();
    let mut app = App::new(store::load().unwrap_or_default());
    let mut frame = Frame::new();
    let mut ticker = Ticker::every(Duration::from_hz(anim::FPS as u64));

    loop {
        while let Ok(ev) = EVENTS.try_receive() {
            if ev == Event::Bootloader {
                // show it for a second, so a held chord is visibly accepted
                for t in 0..anim::FPS {
                    frame.clear();
                    anim::usb_boot(&mut frame, t);
                    display.show(&frame, app.brightness().max(32));
                    ticker.next().await;
                }
                bootloader::reboot_to_boot_rom();
            }
            app.handle(ev);
        }
        if let Some(name) = app.saved_name() {
            store::save(&name); // a page erase and program: a few ms
        }
        frame.clear();
        app.render(&mut frame, touch::held());
        display.show(&frame, app.brightness());
        ticker.next().await;
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
