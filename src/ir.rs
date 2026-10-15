//! IR send and receive; same packets and timing as `c/src/ir.c`. SysTick
//! runs free for the main loop (main.rs `ticks()`), so sending uses TIM3
//! where the C uses the SysTick compare.
//!
//! Send: the LED's anode is on VCC through R37 (200 ohm, about 15 mA) and
//! its cathode on PB21, so it is on while PB21 is low (hardware commit
//! 0986add). TIM2 channel 1 makes the 38 kHz carrier (1/3 duty) there,
//! output inverted so PB21 idles high; TIM2_RM = 001 puts CH1 on PB21
//! (CH643RM 8.3.2.1; CH2..CH4 then land on LED matrix pins, so only CH1 is
//! enabled). TIM3 runs one-shot at 1 MHz for each mark and space; its
//! update interrupt switches the carrier on or off for the next one (the
//! compare value goes through TIM2's preload, so every edge is late by the
//! same part of a carrier period).
//!
//! Receive: as in C, an EXTI interrupt on both edges of PB19 (line 19)
//! stores how long each mark and space lasted, timed with SysTick, in a
//! ring; `poll()` decodes them. The receiver hears our own LED, so
//! receiving pauses while we send and for 3 ms after, unless the bring-up
//! test asks for loopback.
//!
//! Not tested on hardware.

use core::cell::UnsafeCell;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU8, Ordering};

use crate::irproto::{self, Decoder, PACKET_MAX};
use crate::{ticks, ticks_to_us, TICKS_PER_MS};

const CARRIER_HZ: u32 = 38_000;
const ECHO_MS: u32 = 3;

// register offsets (TIM_TypeDef), 16-bit registers on a 4-byte stride
const CTLR1: usize = 0x00;
const DMAINTENR: usize = 0x0C;
const INTFR: usize = 0x10;
const SWEVGR: usize = 0x14;
const CHCTLR1: usize = 0x18;
const CCER: usize = 0x20;
const CNT: usize = 0x24;
const PSC: usize = 0x28;
const ATRLR: usize = 0x2C;
const CH1CVR: usize = 0x34;
const BDTR: usize = 0x44;

const CEN: u16 = 0x0001;
const URS: u16 = 0x0004;
const OPM: u16 = 0x0008;
const ARPE: u16 = 0x0080;
const UIE: u16 = 0x0001;
const UIF: u16 = 0x0001;
const UG: u16 = 0x0001;
const OC1PE: u16 = 0x0008;
const CC1E: u16 = 0x0001;
const CC1P: u16 = 0x0002;
const MOE: u16 = 0x8000;

const TIM2: usize = 0x4000_0000;
const TIM3: usize = 0x4000_0400;
const AFIO_PCFR1: usize = 0x4001_0004;
const AFIO_EXTICR2: usize = 0x4001_000C; // EXTICR[1]: lines 16..23
const EXTI: usize = 0x4001_0400;
const EXTI_INTENR: usize = 0x00;
const EXTI_RTENR: usize = 0x08;
const EXTI_FTENR: usize = 0x0C;
const EXTI_INTFR: usize = 0x14;
const GPIOB: usize = 0x4001_0C00;
const GPIO_INDR: usize = 0x08;
const GPIO_CFGXR: usize = 0x1C;
const GPIO_BSXR: usize = 0x20;

const LED_PIN: u32 = 21; // PB21, LED cathode: TIM2 CH1, low = LED on
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
// bit 15 = mark, then microseconds; written by the EXTI interrupt only
static RX_RING: Shared<[u16; RING]> = Shared(UnsafeCell::new([0; RING]));
static RX_HEAD: AtomicU8 = AtomicU8::new(0);
static RX_TAIL: AtomicU8 = AtomicU8::new(0);
static RX_LAST: AtomicU32 = AtomicU32::new(0); // ticks() at the last edge; the EXTI interrupt's only
static MUTE_UNTIL: AtomicU32 = AtomicU32::new(0); // ticks(): our own echo is over

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
        // done: carrier off (PB21 high), both timers stopped
        wr(TIM2, CH1CVR, 0);
        wr(TIM2, SWEVGR, UG);
        wr(TIM2, CTLR1, 0);
        MUTE_UNTIL.store(ticks().wrapping_add(ECHO_MS * TICKS_PER_MS), Ordering::Relaxed);
        MUTED.store(!LOOPBACK.load(Ordering::Relaxed), Ordering::Relaxed);
        BUSY.store(false, Ordering::Release);
        return;
    };
    wr(TIM2, CH1CVR, if mark { CARRIER_ON.load(Ordering::Relaxed) } else { 0 });
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
fn EXTI25_16() {
    let now = ticks();
    wr32(EXTI + EXTI_INTFR, 1 << RECV_PIN);
    let low = rd32(GPIOB + GPIO_INDR) & 1 << RECV_PIN == 0;
    let us = ticks_to_us(now.wrapping_sub(RX_LAST.load(Ordering::Relaxed)));
    RX_LAST.store(now, Ordering::Relaxed);
    if !LOOPBACK.load(Ordering::Relaxed) && (BUSY.load(Ordering::Relaxed) || MUTED.load(Ordering::Relaxed)) {
        return;
    }
    // the interval that just ended: a space if the output is low now
    let v = us.min(0x7FFF) as u16 | if low { 0 } else { 0x8000 };
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
        let hclk = crate::HCLK;
        crate::pac::RCC.apb2pcenr().modify(|w| {
            w.set_iopben(true);
            w.set_afioen(true);
        });
        crate::pac::RCC.apb1pcenr().modify(|w| {
            w.set_tim2en(true);
            w.set_tim3en(true);
        });
        // TIM2_RM = 001 (CH1 on PB21)
        let pcfr1 = rd32(AFIO_PCFR1) & !(0x7 << 18);
        wr32(AFIO_PCFR1, pcfr1 | 0b001 << 18);

        // PB21 high first (LED off) and alternate push-pull (0b1011), PB19
        // floating input (R35 pulls it up), PB23 (no longer used) input
        // with pull-up
        wr32(GPIOB + GPIO_BSXR, 1 << (LED_PIN - 16) | 1 << (23 - 16));
        let cfg = rd32(GPIOB + GPIO_CFGXR) & !0xF0F0_F000;
        wr32(GPIOB + GPIO_CFGXR, cfg | 0x80B0_4000);

        // TIM2: 38 kHz PWM mode 1 with preload on CH1, active low (CC1P):
        // the LED is on while the counter is below the compare value, so
        // off while it is 0; the counter only runs while sending
        let period = (hclk / CARRIER_HZ) as u16;
        CARRIER_ON.store(period / 3, Ordering::Relaxed);
        wr(TIM2, PSC, 0);
        wr(TIM2, ATRLR, period - 1);
        wr(TIM2, CHCTLR1, (rd(TIM2, CHCTLR1) & 0xFF00) | 0x0060 | OC1PE);
        wr(TIM2, CH1CVR, 0);
        wr(TIM2, CCER, (rd(TIM2, CCER) & !0x000F) | CC1E | CC1P);
        // TIM2 is an advanced timer on the CH643 (RM ch. 12, "TIM1/TIM2"):
        // no output at all until the main output enable is set
        wr(TIM2, BDTR, MOE);
        wr(TIM2, SWEVGR, UG);

        // TIM3: 1 MHz, one-shot per segment, update interrupt
        let us_psc = (hclk / 1_000_000 - 1) as u16;
        wr(TIM3, PSC, us_psc);
        wr(TIM3, SWEVGR, UG); // load the prescaler
        wr(TIM3, INTFR, 0);
        wr(TIM3, DMAINTENR, UIE);

        // EXTI line 19 from port B (EXTICR field 0b01), both edges
        wr32(AFIO_EXTICR2, rd32(AFIO_EXTICR2) & !(0b11 << 6) | 0b01 << 6);
        wr32(EXTI + EXTI_RTENR, rd32(EXTI + EXTI_RTENR) | 1 << RECV_PIN);
        wr32(EXTI + EXTI_FTENR, rd32(EXTI + EXTI_FTENR) | 1 << RECV_PIN);
        wr32(EXTI + EXTI_INTFR, 1 << RECV_PIN);
        wr32(EXTI + EXTI_INTENR, rd32(EXTI + EXTI_INTENR) | 1 << RECV_PIN);
        RX_LAST.store(ticks(), Ordering::Relaxed);

        unsafe {
            use crate::pac::Interrupt;
            qingke::pfic::enable_interrupt(Interrupt::TIM3 as u8);
            qingke::pfic::enable_interrupt(Interrupt::EXTI25_16 as u8);
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
        wr(TIM2, CNT, 0);
        wr(TIM2, CTLR1, ARPE | CEN);
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
        if MUTED.load(Ordering::Relaxed)
            && ticks().wrapping_sub(MUTE_UNTIL.load(Ordering::Relaxed)) as i32 >= 0
        {
            MUTED.store(false, Ordering::Relaxed);
            self.dec.reset();
        }
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
