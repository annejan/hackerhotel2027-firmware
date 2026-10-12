//! Capacitive touch pads T1..T4 on PC0..PC3 (ADC/TouchKey channels 10..13).
//!
//! The CH643 TouchKey unit is the ADC with a charge/discharge front end.
//! Sequence copied from WCH's EVT example (EXAM/TOUCHKEY/TKey/User/main.c):
//! enable CTLR1.TKENABLE once, then per measurement select the channel,
//! write the charge time to IDATAR1 and the discharge time to RDATAR (this
//! starts the measurement), wait for EOC and read RDATAR.
//!
//! Datasheet note: ADC channels 3, 7, 11 and 15 are missing on chips whose
//! lot number has 0 as the 5th digit from the end. T2 sits on channel 11,
//! so on those chips T2 never fires.

use ch32_hal as hal;
use embassy_time::{Duration, Ticker};
use hal::adc::{Adc, AdcChannel, AnyAdcChannel, SampleTime};
use hal::peripherals::{ADC1, PC0, PC1, PC2, PC3};
use hal::Peri;

use crate::{Event, EVENTS};

const PADS: usize = 4;
const POLL_MS: u64 = 10;
pub const HOLD_MS: u64 = 600;
const HOLD_TICKS: u16 = (HOLD_MS / POLL_MS) as u16;

// From the WCH example.
const CHARGE_TIME: u32 = 0x80;
const DISCHARGE_TIME: u32 = 0x08;

// Untuned: depends on pad size, solder mask and the case. Log raw values
// over SDI print on a real board and set these.
const PRESS_DELTA: u32 = 80;
const RELEASE_DELTA: u32 = 40;

// ADC register offsets (ADC_TypeDef in ch643.h). The metapac marks IDATAR
// read-only, but TouchKey writes the charge time there, so poke it directly.
const IDATAR1: usize = 0x3C;
const RDATAR: usize = 0x4C;

struct Pad {
    /// Untouched level, 4 fractional bits.
    baseline: u32,
    down: bool,
    ticks: u16,
}

fn measure(adc: &mut Adc<'_, ADC1>, ch: &mut AnyAdcChannel<ADC1>) -> u32 {
    adc.configure_channel(ch, 1, SampleTime::CYCLES11);
    let base = hal::pac::ADC1.as_ptr() as usize;
    unsafe {
        core::ptr::write_volatile((base + IDATAR1) as *mut u32, CHARGE_TIME);
        core::ptr::write_volatile((base + RDATAR) as *mut u32, DISCHARGE_TIME);
    }
    while !hal::pac::ADC1.statr().read().eoc() {}
    unsafe { core::ptr::read_volatile((base + RDATAR) as *const u32) & 0x0FFF }
}

#[embassy_executor::task]
pub async fn scan(
    adc: Peri<'static, ADC1>,
    t1: Peri<'static, PC0>,
    t2: Peri<'static, PC1>,
    t3: Peri<'static, PC2>,
    t4: Peri<'static, PC3>,
) {
    let mut adc = Adc::new(adc, Default::default());
    hal::pac::ADC1.ctlr1().modify(|w| w.set_tkenable(true));

    let mut chans = [t1.degrade_adc(), t2.degrade_adc(), t3.degrade_adc(), t4.degrade_adc()];
    let mut pads: [Pad; PADS] = core::array::from_fn(|_| Pad { baseline: 0, down: false, ticks: 0 });
    for (pad, ch) in pads.iter_mut().zip(chans.iter_mut()) {
        pad.baseline = measure(&mut adc, ch) << 4;
    }

    let mut ticker = Ticker::every(Duration::from_millis(POLL_MS));
    loop {
        for (i, (pad, ch)) in pads.iter_mut().zip(chans.iter_mut()).enumerate() {
            let raw = measure(&mut adc, ch);
            let base = pad.baseline >> 4;
            // A finger adds capacitance; which way the count moves is not
            // documented, so react to either direction.
            let delta = raw.abs_diff(base);

            if !pad.down {
                if delta > PRESS_DELTA {
                    pad.down = true;
                    pad.ticks = 0;
                } else {
                    // follow slow drift (temperature, humidity) while untouched
                    pad.baseline = pad.baseline - (pad.baseline >> 5) + (raw << 4 >> 5);
                }
            } else if delta < RELEASE_DELTA {
                pad.down = false;
                if pad.ticks < HOLD_TICKS {
                    let _ = EVENTS.try_send(Event::Tap(i as u8));
                }
            } else {
                pad.ticks = pad.ticks.saturating_add(1);
                if pad.ticks == HOLD_TICKS {
                    let _ = EVENTS.try_send(Event::Hold(i as u8));
                }
            }
        }
        ticker.next().await;
    }
}
