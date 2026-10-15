//! CH643 LEDPWM matrix driver, from the CH643 reference manual (CH643RM
//! V1.9, chapter 24). Same as `c/src/ledpwm.c`. NOT TESTED ON HARDWARE.
//!
//! The board is the manual's "8 columns RGB, built-in drive" case (24.2.2;
//! reference circuit 24.4.2 is 16 rows x 8 RGB columns, like this badge):
//! PA0..PA23 are the PWM channels (the chip sources current, active high,
//! through the series resistors to the anodes), PB0..PB15 the COM lines
//! (built-in NMOS sinks the cathodes, active low). In MATRIX_MODE PA24 the
//! PWM pins form 8 columns of three: column k is PA k ("red"), PA k+8
//! ("green") and PA k+16 ("blue"). Here they are simply 24 channels.
//!
//! The scanner does not run by itself: it shows one COM line, stops and
//! raises its interrupt. The handler points the DMA at the next row (8
//! words; per column the intensity byte, then the PA k, PA k+8, PA k+16
//! levels), writes DMA_CNT = 8, selects the next COM (which clears the
//! flag) and sets PWM_OE again (PWM_OE_CLR switched it off for the change,
//! so the rows do not ghost into each other).
//!
//! Timing: intensity PWM 6 bits (64 clocks), colour PWM 8 bits (256
//! intensity periods): 16384 clocks a row, 341 us at 48 MHz, 5.5 ms for
//! the 16 rows, 183 Hz. The brightness goes in the intensity byte, so the
//! badge dims without losing any of the 256 colour steps.
//!
//! GUESS (the manual does not say): the LED reference clock is HCLK; with
//! the 6-bit intensity PWM the intensity byte's low 6 bits count; DMA
//! addresses are the low 16 bits of a RAM address.

use core::ptr::{addr_of, addr_of_mut, write_volatile};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};


use crate::frame::Frame;
use crate::generated::{COMS, GAMMA, PWM_CHANNELS};

const BASE: usize = 0x4002_6800;

// byte offsets, CH643RM 24.3
const COM_SEL: usize = 0x00;
const KEY_CTRL: usize = 0x01;
const PWM_OE: usize = 0x02;
const DMA_CNT: usize = 0x03;
const COM_E_PB_L: usize = 0x04;
const COM_E_PB_M: usize = 0x05;
const COM_E_PB_H: usize = 0x06;
const COM_E_PAPC: usize = 0x07;
const PWM_PIN_L: usize = 0x08;
const PWM_PIN_H: usize = 0x09;
const PWM_MOD: usize = 0x0B;
const FUNC_CTRL: usize = 0x0C;
const ANA_CTRL: usize = 0x0D;
const CYCLE_CFG: usize = 0x0E;
const FRAME_CFG: usize = 0x0F;
const DMA_ADDR: usize = 0x18; // 16-bit

const COM_SEL_PB0: u8 = 0x10; // PBn is 0x10 + n
const MATRIX_PA24: u8 = 0x01; // PA[7:0] / PA[15:8] / PA[23:16]
const PWM_RGB_EN: u8 = 0x70; // red, green and blue groups on
const INTEN_6BIT: u8 = 0x20; // 6-bit intensity, 8-bit colour, clock / 1
const INHIBIT_4: u8 = 3 << 4; // 4 intensity periods of blanking between rows
const IE_INHIBIT: u8 = 0x80; // interrupt at the end of each row
const PWM_OE_CLR: u8 = 0x04; // clear PWM_OE on that interrupt
const LED_ENABLE: u8 = 0x01; // the LED clock

const ROW_WORDS: usize = 8;

// GPIO offsets (CFGLR, CFGHR, BSHR, BCR, CFGXR)
const GPIO_CFGLR: usize = 0x00;
const GPIO_CFGHR: usize = 0x04;
const GPIO_BSHR: usize = 0x10;
const GPIO_BCR: usize = 0x14;
const GPIO_CFGXR: usize = 0x1C;

/// Two frames of row data: the scanner reads `FRONT`, `show()` fills the
/// other one, and the interrupt swaps them at the start of a scan.
#[repr(align(4))]
struct Rows([[[u32; ROW_WORDS]; COMS]; 2]);
static mut ROWS: Rows = Rows([[[0; ROW_WORDS]; COMS]; 2]);
// load/store only: no CAS on this core without the A extension in the target
static FRONT: AtomicU8 = AtomicU8::new(0);
static COM: AtomicU8 = AtomicU8::new(0);
static PENDING: AtomicBool = AtomicBool::new(false);
static ROWS_SHOWN: AtomicU32 = AtomicU32::new(0);

/// Rows shown since power on (for the bring-up test).
pub fn rows() -> u32 {
    ROWS_SHOWN.load(Ordering::Relaxed)
}

#[inline]
fn w8(off: usize, v: u8) {
    unsafe { write_volatile((BASE + off) as *mut u8, v) }
}

#[inline]
fn w32(addr: usize, v: u32) {
    unsafe { write_volatile(addr as *mut u32, v) }
}

fn load_row(c: u8) {
    let front = FRONT.load(Ordering::Relaxed) as usize;
    let row = unsafe { addr_of!(ROWS.0[front][c as usize]) };
    unsafe { write_volatile((BASE + DMA_ADDR) as *mut u16, row as usize as u16) };
    w8(DMA_CNT, ROW_WORDS as u8);
    w8(COM_SEL, COM_SEL_PB0 + c); // clears the interrupt flag
    w8(PWM_OE, 1);
}

#[qingke_rt::interrupt]
fn LEDPWM() {
    ROWS_SHOWN.store(ROWS_SHOWN.load(Ordering::Relaxed).wrapping_add(1), Ordering::Relaxed);
    let mut c = COM.load(Ordering::Relaxed) + 1;
    if c as usize == COMS {
        c = 0;
        if PENDING.load(Ordering::Relaxed) {
            FRONT.store(FRONT.load(Ordering::Relaxed) ^ 1, Ordering::Relaxed);
            PENDING.store(false, Ordering::Release);
        }
    }
    COM.store(c, Ordering::Relaxed);
    load_row(c);
}

pub struct LedPwm {
    /// The last frame never got picked up: the scanner is not running.
    stalled: bool,
}

impl LedPwm {
    pub fn new() -> Self {
        crate::pac::RCC.apb2pcenr().modify(|w| {
            w.set_iopaen(true);
            w.set_iopben(true);
        });
        // PWM pins PA0..PA23: push-pull outputs, OUTDR 0 = active high.
        // COM pins PB0..PB15: push-pull outputs, OUTDR 1 = active low.
        // (RM 24.2.2 and table 8-10; with the LED block off this also
        // keeps every LED dark.)
        let (pa, pb) = (crate::pac::GPIOA.as_ptr() as usize, crate::pac::GPIOB.as_ptr() as usize);
        w32(pa + GPIO_BCR, 0x00FF_FFFF);
        w32(pa + GPIO_CFGLR, 0x3333_3333);
        w32(pa + GPIO_CFGHR, 0x3333_3333);
        w32(pa + GPIO_CFGXR, 0x3333_3333);
        w32(pb + GPIO_BSHR, 0xFFFF);
        w32(pb + GPIO_CFGLR, 0x3333_3333);
        w32(pb + GPIO_CFGHR, 0x3333_3333);

        w8(COM_E_PB_L, 0xFF); // PB0..PB15 are COM lines
        w8(COM_E_PB_M, 0xFF);
        w8(COM_E_PB_H, 0x00);
        w8(COM_E_PAPC, 0x00);
        w8(PWM_PIN_L, 0xFF); // columns 0..7
        w8(PWM_PIN_H, 0x00);
        w8(PWM_MOD, PWM_RGB_EN | MATRIX_PA24);
        w8(ANA_CTRL, 0x00); // no built-in constant current: the board has resistors
        w8(CYCLE_CFG, INTEN_6BIT);
        w8(FRAME_CFG, INHIBIT_4); // ~5 us between rows, no repeats
        w8(KEY_CTRL, 0x00);
        w8(FUNC_CTRL, IE_INHIBIT | PWM_OE_CLR | LED_ENABLE);
        // row 0 first, then the interrupt: loading the row clears a flag the
        // idle block may already have raised, so the handler cannot run
        // before it and start on row 1 while row 0 is being loaded
        COM.store(0, Ordering::Relaxed);
        load_row(0);
        unsafe { qingke::pfic::enable_interrupt(crate::pac::Interrupt::LEDPWM as u8) };
        Self { stalled: false }
    }

    /// Gamma-correct `frame` and hand it to the scanner; `brightness`
    /// (0..=255) sets the intensity PWM.
    pub fn show(&mut self, frame: &Frame, brightness: u8) {
        // the last frame is still waiting for the start of a scan (5.5 ms at
        // most); give up after a while instead of hanging if the scanner
        // stopped, and once it has stopped do not wait again: every 30 ms
        // spent spinning here would starve the other tasks
        if !self.stalled {
            for _ in 0..200_000 {
                if !PENDING.load(Ordering::Acquire) {
                    break;
                }
            }
        }
        self.stalled = PENDING.load(Ordering::Acquire);
        let inten = match brightness >> 2 {
            0 if brightness > 0 => 1,
            v => v as u32,
        };
        let back = FRONT.load(Ordering::Relaxed) as usize ^ 1;
        let rows = unsafe { &mut (*addr_of_mut!(ROWS)).0[back] };
        for (c, row) in rows.iter_mut().enumerate() {
            let px = &frame.px[c * PWM_CHANNELS..][..PWM_CHANNELS];
            for (k, word) in row.iter_mut().enumerate() {
                *word = inten
                    | (GAMMA[px[k] as usize] as u32) << 8
                    | (GAMMA[px[k + 8] as usize] as u32) << 16
                    | (GAMMA[px[k + 16] as usize] as u32) << 24;
            }
        }
        PENDING.store(true, Ordering::Release);
    }
}
