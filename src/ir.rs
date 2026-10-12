//! IR send and receive; same packets and timing as `c/src/ir.c`, other
//! timers: Embassy owns SysTick here.
//!
//! Send: TIM1 channel 4 makes the 38 kHz carrier (1/3 duty) on PB23, the
//! LED's anode (TIM1_RM = 1xx, CH643RM 8.3.2.1); the cathode PB21 stays
//! low. TIM3 runs one-shot at 1 MHz for each mark and space; its update
//! interrupt switches the carrier on or off for the next one (the compare
//! value goes through TIM1's preload, so every edge is late by the same
//! part of a carrier period).
//!
//! Receive: TIM2 channel 4 captures the receiver's edges on PB19 (TIM2_RM =
//! 100) at 1 MHz; the capture interrupt stores how long each mark and space
//! lasted in a ring, and the update interrupt counts wraps so long gaps are
//! not mistaken for short ones. `poll()` decodes them. The receiver hears
//! our own LED, so receiving pauses while we send and for 3 ms after,
//! unless the bring-up test asks for loopback.
//!
//! Not tested on hardware.

use core::cell::UnsafeCell;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering};

use ch32_hal as hal;

use crate::irproto::{self, Decoder, PACKET_MAX};

const CARRIER_HZ: u32 = 38_000;
const ECHO_US: u16 = 3000;

// register offsets (TIM_TypeDef), 16-bit registers on a 4-byte stride
const CTLR1: usize = 0x00;
const DMAINTENR: usize = 0x0C;
const INTFR: usize = 0x10;
const SWEVGR: usize = 0x14;
const CHCTLR2: usize = 0x1C;
const CCER: usize = 0x20;
const CNT: usize = 0x24;
const PSC: usize = 0x28;
const ATRLR: usize = 0x2C;
const CH4CVR: usize = 0x40;
const BDTR: usize = 0x44;

const CEN: u16 = 0x0001;
const URS: u16 = 0x0004;
const OPM: u16 = 0x0008;
const ARPE: u16 = 0x0080;
const UIE: u16 = 0x0001;
const CC4IE: u16 = 0x0010;
const UIF: u16 = 0x0001;
const UG: u16 = 0x0001;
const OC4PE: u16 = 0x0800;
const CC4E: u16 = 0x1000;
const CC4P: u16 = 0x2000;
const MOE: u16 = 0x8000;

const TIM1: usize = 0x4001_2C00;
const TIM2: usize = 0x4000_0000;
const TIM3: usize = 0x4000_0400;
const AFIO_PCFR1: usize = 0x4001_0004;
const GPIOB: usize = 0x4001_0C00;
const GPIO_INDR: usize = 0x08;
const GPIO_BCR: usize = 0x14;
const GPIO_CFGXR: usize = 0x1C;

const CATHODE_PIN: u32 = 21; // PB21, LED cathode: always low
const RECV_PIN: u32 = 19; // PB19, receiver output, low while it sees carrier

#[inline]
fn rd(base: usize, off: usize) -> u16 {
    unsafe { read_volatile((base + off) as *const u16) }
}

#[inline]
fn wr(base: usize, off: usize, v: u16) {
    unsafe { write_volatile((base + off) as *mut u16, v) }
}

#[inline]
fn rd32(addr: usize) -> u32 {
    unsafe { read_volatile(addr as *const u32) }
}

#[inline]
fn wr32(addr: usize, v: u32) {
    unsafe { write_volatile(addr as *mut u32, v) }
}

/// State shared with the interrupts. The send side is only touched by the
/// TIM3 interrupt while BUSY is set and by `send()` while it is clear.
struct Tx {
    buf: [u8; PACKET_MAX],
    seg: u16,
    segs: u16,
}
struct Shared<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Shared<T> {}

static TX: Shared<Tx> = Shared(UnsafeCell::new(Tx { buf: [0; PACKET_MAX], seg: 0, segs: 0 }));
static BUSY: AtomicBool = AtomicBool::new(false);
static CARRIER_ON: AtomicU16 = AtomicU16::new(0);
static LOOPBACK: AtomicBool = AtomicBool::new(false);
static MUTED: AtomicBool = AtomicBool::new(false);

const RING: usize = 128;
// bit 15 = mark, then microseconds; written by the capture interrupt only
static RX_RING: Shared<[u16; RING]> = Shared(UnsafeCell::new([0; RING]));
static RX_HEAD: AtomicU8 = AtomicU8::new(0);
static RX_TAIL: AtomicU8 = AtomicU8::new(0);
static RX_LAST: AtomicU16 = AtomicU16::new(0); // TIM2 count at the last edge
static RX_WRAPS: AtomicU8 = AtomicU8::new(0); // TIM2 wraps since the last edge
static MUTE_LEFT: AtomicU16 = AtomicU16::new(0); // us of echo still to ignore

/// Segments: 0 header mark, 1 header space, then a mark and a space per
/// bit, then the end mark. Runs in the TIM3 interrupt.
fn next_segment() {
    let tx = unsafe { &mut *TX.0.get() };
    let s = tx.seg;
    tx.seg += 1;
    let (mark, us) = if s == 0 {
        (true, irproto::HDR_MARK_US)
    } else if s == 1 {
        (false, irproto::HDR_SPACE_US)
    } else if s < tx.segs {
        let k = ((s - 2) / 2) as usize;
        let mark = s & 1 == 0;
        let one = !mark && tx.buf[k / 8] >> (k % 8) & 1 != 0;
        (mark, if mark { irproto::BIT_MARK_US } else if one { irproto::ONE_SPACE_US } else { irproto::ZERO_SPACE_US })
    } else {
        // done: carrier off, both timers stopped
        wr(TIM1, CH4CVR, 0);
        wr(TIM1, SWEVGR, UG);
        wr(TIM1, CTLR1, 0);
        MUTE_LEFT.store(ECHO_US, Ordering::Relaxed);
        MUTED.store(!LOOPBACK.load(Ordering::Relaxed), Ordering::Relaxed);
        BUSY.store(false, Ordering::Release);
        return;
    };
    wr(TIM1, CH4CVR, if mark { CARRIER_ON.load(Ordering::Relaxed) } else { 0 });
    wr(TIM3, ATRLR, (us - 1) as u16);
    wr(TIM3, CNT, 0);
    wr(TIM3, CTLR1, OPM | URS | CEN);
}

#[qingke_rt::interrupt]
fn TIM3() {
    wr(TIM3, INTFR, !UIF);
    next_segment();
}

#[qingke_rt::interrupt]
fn TIM2_UP() {
    wr(TIM2, INTFR, !UIF);
    let w = RX_WRAPS.load(Ordering::Relaxed);
    RX_WRAPS.store(w.saturating_add(1), Ordering::Relaxed);
    // the echo window runs out with time, edges or not
    let left = MUTE_LEFT.load(Ordering::Relaxed);
    if left > 0 {
        MUTE_LEFT.store(0, Ordering::Relaxed);
        MUTED.store(false, Ordering::Relaxed);
    }
}

#[qingke_rt::interrupt]
fn TIM2_CC() {
    let now = rd(TIM2, CH4CVR); // reading it clears CC4IF
    let low = rd32(GPIOB + GPIO_INDR) & 1 << RECV_PIN == 0;
    // catch the next edge the other way round
    let ccer = rd(TIM2, CCER);
    wr(TIM2, CCER, if low { ccer & !CC4P } else { ccer | CC4P });
    // load and store only (no atomic swap on this core); TIM2's two
    // interrupts have the same priority, so they never interrupt each other
    let last = RX_LAST.load(Ordering::Relaxed);
    RX_LAST.store(now, Ordering::Relaxed);
    let wraps = RX_WRAPS.load(Ordering::Relaxed);
    RX_WRAPS.store(0, Ordering::Relaxed);
    let dt = now.wrapping_sub(last) as u32;
    let us = if wraps > 1 || (wraps == 1 && now >= last) { 0x7FFF } else { dt.min(0x7FFF) };
    if MUTED.load(Ordering::Relaxed) {
        let left = MUTE_LEFT.load(Ordering::Relaxed);
        if us >= left as u32 {
            MUTE_LEFT.store(0, Ordering::Relaxed);
            MUTED.store(false, Ordering::Relaxed);
        } else {
            MUTE_LEFT.store(left - us as u16, Ordering::Relaxed);
        }
        return;
    }
    if BUSY.load(Ordering::Relaxed) && !LOOPBACK.load(Ordering::Relaxed) {
        return;
    }
    // the interval that just ended: a space if the output is low now
    let v = us as u16 | if low { 0 } else { 0x8000 };
    let head = RX_HEAD.load(Ordering::Relaxed);
    let next = ((head as usize + 1) % RING) as u8;
    if next != RX_TAIL.load(Ordering::Acquire) {
        unsafe { (*RX_RING.0.get())[head as usize] = v };
        RX_HEAD.store(next, Ordering::Release);
    }
}

pub struct Ir {
    dec: Decoder,
}

impl Ir {
    pub fn new() -> Self {
        let hclk = hal::rcc::clocks().hclk.0;
        hal::pac::RCC.apb2pcenr().modify(|w| {
            w.set_iopben(true);
            w.set_afioen(true);
            w.set_tim1en(true);
        });
        hal::pac::RCC.apb1pcenr().modify(|w| {
            w.set_tim2en(true);
            w.set_tim3en(true);
        });
        // TIM1_RM = 100 (CH4 on PB23), TIM2_RM = 100 (CH4 on PB19)
        let pcfr1 = rd32(AFIO_PCFR1) & !(0x7 << 15 | 0x7 << 18);
        wr32(AFIO_PCFR1, pcfr1 | 0b100 << 15 | 0b100 << 18);

        // PB21 push-pull output low, PB23 alternate push-pull (0b1011),
        // PB19 floating input (R35 pulls it up)
        wr32(GPIOB + GPIO_BCR, 1 << CATHODE_PIN);
        let cfg = rd32(GPIOB + GPIO_CFGXR) & !0xF0F0_F000;
        wr32(GPIOB + GPIO_CFGXR, cfg | 0xB030_4000);

        // TIM1: 38 kHz PWM mode 1 with preload on CH4, low while the
        // compare value is 0; the counter only runs while sending
        let period = (hclk / CARRIER_HZ) as u16;
        CARRIER_ON.store(period / 3, Ordering::Relaxed);
        wr(TIM1, PSC, 0);
        wr(TIM1, ATRLR, period - 1);
        wr(TIM1, CHCTLR2, (rd(TIM1, CHCTLR2) & 0x00FF) | 0x6000 | OC4PE);
        wr(TIM1, CH4CVR, 0);
        wr(TIM1, CCER, rd(TIM1, CCER) | CC4E);
        wr(TIM1, BDTR, rd(TIM1, BDTR) | MOE);
        wr(TIM1, SWEVGR, UG);

        // TIM3: 1 MHz, one-shot per segment, update interrupt
        let us_psc = (hclk / 1_000_000 - 1) as u16;
        wr(TIM3, PSC, us_psc);
        wr(TIM3, SWEVGR, UG); // load the prescaler
        wr(TIM3, INTFR, 0);
        wr(TIM3, DMAINTENR, UIE);

        // TIM2: free-running at 1 MHz, CH4 input capture (CC4S = 01: TI4),
        // first edge falling (the receiver idles high)
        wr(TIM2, PSC, us_psc);
        wr(TIM2, ATRLR, 0xFFFF);
        wr(TIM2, CHCTLR2, (rd(TIM2, CHCTLR2) & 0x00FF) | 0x0100);
        wr(TIM2, CCER, rd(TIM2, CCER) | CC4E | CC4P);
        wr(TIM2, SWEVGR, UG);
        wr(TIM2, INTFR, 0);
        wr(TIM2, DMAINTENR, CC4IE | UIE);
        wr(TIM2, CTLR1, ARPE | CEN);

        unsafe {
            use hal::pac::Interrupt;
            qingke::pfic::enable_interrupt(Interrupt::TIM3 as u8);
            qingke::pfic::enable_interrupt(Interrupt::TIM2_CC as u8);
            qingke::pfic::enable_interrupt(Interrupt::TIM2_UP as u8);
        }
        Self { dec: Decoder::new() }
    }

    pub fn busy(&self) -> bool {
        BUSY.load(Ordering::Acquire)
    }

    /// Start sending a packet (irproto::pack format); false while one is
    /// still going out.
    pub fn send(&mut self, ty: u8, data: &[u8]) -> bool {
        if self.busy() {
            return false;
        }
        let tx = unsafe { &mut *TX.0.get() };
        let n = irproto::pack(ty, data, &mut tx.buf);
        if n == 0 {
            return false;
        }
        tx.seg = 0;
        tx.segs = (2 + n * 8 * 2 + 1) as u16;
        BUSY.store(true, Ordering::Release);
        wr(TIM1, CNT, 0);
        wr(TIM1, CTLR1, ARPE | CEN);
        next_segment(); // the header mark; TIM3 takes it from there
        true
    }

    /// True: also receive what we send (the bring-up test).
    pub fn set_loopback(&mut self, on: bool) {
        if LOOPBACK.load(Ordering::Relaxed) != on {
            LOOPBACK.store(on, Ordering::Relaxed);
            self.dec.reset();
        }
    }

    /// Decode what came in; returns a packet (type, data) when one is
    /// complete. Call it every few ms.
    pub fn poll(&mut self, mut got: impl FnMut(u8, &[u8])) {
        loop {
            let tail = RX_TAIL.load(Ordering::Relaxed);
            if tail == RX_HEAD.load(Ordering::Acquire) {
                return;
            }
            let v = unsafe { (*RX_RING.0.get())[tail as usize] };
            RX_TAIL.store(((tail as usize + 1) % RING) as u8, Ordering::Release);
            let n = self.dec.feed(v & 0x8000 != 0, (v & 0x7FFF) as u32);
            if n > 0 {
                let len = self.dec.buf[1] as usize;
                got(self.dec.buf[0], &self.dec.buf[2..2 + len]);
            }
        }
    }
}
