//! CH643 LEDPWM matrix driver. THEORETICAL, NOT TESTED ON HARDWARE.
//!
//! What is known:
//!   * register block at 0x4002_6800 (AHB + 0x6800), interrupt 19, layout and
//!     field names from WCH's `ch643.h` (github.com/openwch/ch643,
//!     EVT/EXAM/SRC/Peripheral/inc/ch643.h)
//!   * datasheet 1.4.18: built-in current-drive PWM channels, COM scanning
//!     1/2 .. 1/18+, 8-bit grey levels, a dedicated DMA that reads display
//!     data from RAM or flash, "fully built-in" mode = PA pins as PWM
//!     (source) and PB pins as COM (sink). That matches this board.
//!
//! What is NOT known: WCH has not published a CH643 reference manual and
//! the EVT has no LEDPWM example, so the meaning of most bit fields below is
//! guessed from their names. Every `GUESS` must be checked against the
//! reference manual (ask WCH for it), WCH's MounRiver LEDPWM library, or a
//! scope on a real board before trusting it.
//!
//! The rest of the firmware only uses `LedPwm::new()` and `show()`, so
//! swapping this file for a GPIO + timer software scanner stays local.

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use crate::frame::{scale, Frame};
use crate::generated::{COMS, GAMMA, PWM_CHANNELS, SLOTS};

const BASE: usize = 0x4002_6800;

// byte offsets, from LEDPWM_TypeDef in ch643.h
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
const DISP_STAT: usize = 0x10;
const INTEN_ADJ: usize = 0x14;
const DMA_ADDR: usize = 0x18; // 16-bit

// bits, from ch643.h
const FUNC_LED_ENABLE: u8 = 0x01;
const KEY_CTRL_COM_LVE: u8 = 0x04;
const PWM_OE_EN: u8 = 0x01;
const DISP_STAT_LED_IF_SET: u8 = 0x40;

/// Display data the LEDPWM DMA reads: one grey byte per slot, COM-major.
/// GUESS: layout (COM-major, PWM_CHANNELS bytes per COM) and that the DMA
/// address register takes the low 16 bits of a RAM address.
static mut DMA_BUF: [u8; SLOTS] = [0; SLOTS];

pub struct LedPwm {
    _private: (),
}

#[inline]
fn w8(off: usize, v: u8) {
    unsafe { write_volatile((BASE + off) as *mut u8, v) }
}

#[inline]
fn r8(off: usize) -> u8 {
    unsafe { read_volatile((BASE + off) as *const u8) }
}

#[inline]
fn w16(off: usize, v: u16) {
    unsafe { write_volatile((BASE + off) as *mut u16, v) }
}

impl LedPwm {
    pub fn new() -> Self {
        // GUESS: no LEDPWM clock-enable bit is defined in ch643.h's RCC section,
        // so the block is assumed to be clocked from HCLK unconditionally.

        // COM lines: PB0..PB15 enabled as scan outputs.
        w8(COM_E_PB_L, 0xFF);
        w8(COM_E_PB_M, 0xFF);
        w8(COM_E_PB_H, 0x00);
        // No COMs on port A or C: in particular COM_PA[23:16]_EN must stay 0,
        // because PA16..PA23 are PWM channels on this board.
        w8(COM_E_PAPC, 0x00);

        // PWM channels. The header only has 16 PWM_PIN bits; PA16..PA23 are
        // presumably the second group selected through PWM_MOD.
        // GUESS: MATRIX_MODE (PWM_MOD[2:0]) value for "24 mono columns, built-in COM".
        w8(PWM_PIN_L, 0xFF);
        w8(PWM_PIN_H, 0xFF);
        w8(PWM_MOD, 0x01);

        // GUESS: COM_SEL = number of COMs - 1, KEY_CTRL.COM_LVE = COM active low
        // (built-in NMOS sinks the cathodes).
        w8(COM_SEL, (COMS - 1) as u8);
        w8(KEY_CTRL, KEY_CTRL_COM_LVE);

        // GUESS: CYCLE_CFG.CLK_FREQ = 0 -> fastest PWM clock, 8-bit grey (COLOR_CYC = 0);
        // FRAME_CFG: no repeat, no inhibit.
        w8(CYCLE_CFG, 0x00);
        w8(FRAME_CFG, 0x00);
        w8(ANA_CTRL, 0x00);
        w8(INTEN_ADJ, 0xFF);

        // GUESS: DMA_CNT = bytes per COM - 1, DMA = RAM address of the buffer.
        w8(DMA_CNT, (PWM_CHANNELS - 1) as u8);
        w16(DMA_ADDR, addr_of!(DMA_BUF) as usize as u16);

        w8(PWM_OE, PWM_OE_EN);
        w8(FUNC_CTRL, FUNC_LED_ENABLE);

        Self { _private: () }
    }

    /// Gamma-correct, scale the duty by `brightness` and hand `frame` to the scanner.
    pub fn show(&mut self, frame: &Frame, brightness: u8) {
        // GUESS: LED_IF_SET flags the end of a scan frame. Copying right after it
        // keeps a frame from tearing; give up after a while instead of hanging
        // if the guess is wrong.
        for _ in 0..2000 {
            if r8(DISP_STAT) & DISP_STAT_LED_IF_SET != 0 {
                w8(DISP_STAT, DISP_STAT_LED_IF_SET); // GUESS: write 1 to clear
                break;
            }
        }
        let buf = unsafe { &mut *addr_of_mut!(DMA_BUF) };
        for (dst, &v) in buf.iter_mut().zip(frame.px.iter()) {
            // gamma first, then brightness scales the PWM duty: scaling before
            // gamma would round night mode down to all-off
            *dst = scale(GAMMA[v as usize], brightness);
        }
    }
}
