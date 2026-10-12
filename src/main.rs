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
//!   touch::scan    polls the touch pads, sends Tap/Hold events
//!   charge_monitor watches CHRG, sends Charging events
#![no_std]
#![no_main]

mod anim;
mod font;
mod frame;
mod generated;
mod ledpwm;
mod touch;

use ch32_hal as hal;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker, Timer};
use hal::gpio::{Input, Pull};

use anim::Player;
use frame::Frame;

/// Things that happen to the badge, consumed by the render loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Short touch on pad 0..=3 (T1..T4)
    Tap(u8),
    /// Touch held for `touch::HOLD_MS` on pad 0..=3
    Hold(u8),
    /// USB power present and battery charging
    Charging(bool),
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

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let mut config = hal::Config::default();
    config.rcc = hal::rcc::Config::SYSCLK_FREQ_48MHZ_HSI;
    let p = hal::init(config);

    spawner.spawn(charge_monitor(Input::new(p.PB22, Pull::Up)).unwrap());
    spawner.spawn(touch::scan(p.ADC1, p.PC0, p.PC1, p.PC2, p.PC3).unwrap());

    // LEDPWM owns PA0..PA23 and PB0..PB15; nothing else may touch those pins.
    let mut display = ledpwm::LedPwm::new();
    let mut player = Player::new();
    let mut frame = Frame::new();
    let mut ticker = Ticker::every(Duration::from_hz(anim::FPS as u64));

    loop {
        while let Ok(ev) = EVENTS.try_receive() {
            player.handle(ev);
        }
        frame.clear();
        player.render(&mut frame);
        display.show(&frame, player.brightness());
        ticker.next().await;
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
