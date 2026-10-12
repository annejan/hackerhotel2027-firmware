//! Hacker Hotel 2027 starfish badge, theoretical Embassy firmware.
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
//!   * IR receiver (PB19) and IR LED (PB23 anode, PB21 cathode): ir.rs
//!   * UART header (USART3: PB16 TX, PB17 RX): uart.rs; SWD header (PC18 SWIO, PC19 SWCK)
//!
//! Tasks:
//!   main loop      renders the current animation at FPS and hands frames to the display
//!   buttons::scan  polls the buttons, sends Press/Tap/Hold/Bootloader/Selftest events
//!   charge_monitor watches CHRG, sends Charging events
//!   infrared       decodes received IR packets, sends the ones the app asks for
//!   serial         the UART: the log out, console lines in
//! The app is shared between the tasks (`with_app`).
#![no_std]
#![no_main]

mod anim;
mod app;
mod bootloader;
mod buttons;
mod console;
mod ir;
mod irproto;
mod log;
mod selftest;
mod uart;
mod editor;
mod font;
mod frame;
mod game;
mod games;
mod generated;
mod ledpwm;
mod nametag;
mod store;

use core::cell::{RefCell, UnsafeCell};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

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

/// The app, shared by the tasks on the one thread-mode executor. No task
/// holds it across an await and no interrupt touches it, so a RefCell is
/// all the locking it needs.
/// (MaybeUninit keeps it in .bss: an `Option<App>` would need 1.2 KB of
/// flash for its initial value.)
struct SharedApp(UnsafeCell<MaybeUninit<RefCell<App>>>, AtomicBool);
unsafe impl Sync for SharedApp {}
static APP: SharedApp = SharedApp(UnsafeCell::new(MaybeUninit::uninit()), AtomicBool::new(false));

fn start_app(app: App) {
    unsafe { (*APP.0.get()).write(RefCell::new(app)) };
    APP.1.store(true, Ordering::Release);
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> R {
    assert!(APP.1.load(Ordering::Acquire), "app not started");
    f(&mut unsafe { (*APP.0.get()).assume_init_ref() }.borrow_mut())
}

/// PC3 sees VBUS (through R33 to GND): high while USB powers the badge.
fn vbus() -> bool {
    hal::pac::GPIOC.indr().read().0 & 1 << 3 != 0
}

#[embassy_executor::task]
async fn infrared() {
    let mut ir = ir::Ir::new();
    loop {
        ir.poll(|ty, data| with_app(|app| app.ir_received(ty, data)));
        with_app(|app| {
            ir.set_loopback(app.ir_loopback());
            if !ir.busy() {
                if let Some((ty, name)) = app.ir_take() {
                    ir.send(ty, name.bytes());
                }
            }
        });
        Timer::after_millis(5).await;
    }
}

#[embassy_executor::task]
async fn serial() {
    let mut uart = uart::Uart::new();
    loop {
        uart.poll(|line| with_app(|app| console::line(app, line)));
        Timer::after_micros(200).await;
    }
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
/// PC0..PC2 are bonded but not connected on this board, so pull those up
/// too. PC3 (VBUS sense, 100k to GND) must stay a floating input.
fn pull_up_unused_pads() {
    use hal::pac::{GPIOC, RCC};
    RCC.apb2pcenr().modify(|w| w.set_iopcen(true));
    // input with pull-up = mode nibble 0b1000, pull selected by OUTDR = 1
    GPIOC.cfglr().modify(|w| w.0 = (w.0 & 0x00FF_F000) | 0x8800_0888); // PC0..PC2, PC6, PC7
    // CFGHR is not readable on early silicon (WCH keeps a shadow copy), so
    // write it whole: PC8, PC9, PC12, PC13 pull-up, the rest floating input
    GPIOC.cfghr().write(|w| w.0 = 0x4488_4488);
    GPIOC.bshr().write(|w| w.0 = 0b111 | 1 << 6 | 1 << 7 | 1 << 8 | 1 << 9 | 1 << 12 | 1 << 13);
    GPIOC.cfgxr().modify(|w| w.0 = (w.0 & !(0xF << 16)) | 0x8 << 16); // PC20
    GPIOC.bsxr().write(|w| w.0 = 1 << 4);
}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    let mut config = hal::Config::default();
    config.rcc = hal::rcc::Config::SYSCLK_FREQ_48MHZ_HSI;
    let p = hal::init(config);
    pull_up_unused_pads();

    spawner.spawn(charge_monitor(Input::new(p.PB22, Pull::Up)).unwrap());
    // ◀ B A ▶; floating inputs, the pull-ups are on the board
    let pins = [
        Input::new(p.PC5, Pull::None),
        Input::new(p.PB20, Pull::None),
        Input::new(p.PC4, Pull::None),
        Input::new(p.PB18, Pull::None),
    ];
    spawner.spawn(buttons::scan(pins).unwrap());

    // LEDPWM owns PA0..PA23 and PB0..PB15; nothing else may touch those pins.
    let mut display = ledpwm::LedPwm::new();
    let name = store::load().unwrap_or_default();
    log::text("\r\nhh27 starfish badge, rust firmware. Name: ");
    log::line(name.as_str());
    log::line("Type help.");
    start_app(App::new(name));
    spawner.spawn(infrared().unwrap());
    spawner.spawn(serial().unwrap());
    let mut frame = Frame::new();
    let mut ticker = Ticker::every(Duration::from_hz(anim::FPS as u64));

    loop {
        while let Ok(ev) = EVENTS.try_receive() {
            if ev == Event::Bootloader {
                // show it for a second, so a held chord is visibly accepted
                let level = with_app(|app| app.brightness()).max(32);
                for t in 0..anim::FPS {
                    frame.clear();
                    anim::usb_boot(&mut frame, t);
                    display.show(&frame, level);
                    ticker.next().await;
                }
                bootloader::reboot_to_boot_rom();
            }
            with_app(|app| app.handle(ev));
        }
        if let Some(name) = with_app(|app| app.saved_name()) {
            store::save(&name); // a page erase and program: a few ms
            log::text("name saved: ");
            log::line(name.as_str());
        }
        frame.clear();
        let level = with_app(|app| {
            app.hw = selftest::Hw { rows: ledpwm::rows(), vbus: vbus() };
            app.render(&mut frame, buttons::held());
            app.brightness()
        });
        display.show(&frame, level);
        ticker.next().await;
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
