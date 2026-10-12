//! USART3 on the UART header J3: pin 1 RX (PB17), pin 2 TX (PB16), pin 3
//! VCC, pin 4 GND; 115200 8N1. Same as `c/src/uart.c`: sends what log.rs
//! collected and hands typed lines to the console. Polled by a task.
//!
//! USART3_RM = 11 puts TX3 / RX3 on PB16 / PB17 (CH643RM 8.3.2.1; CTS, CK
//! and RTS would land on PB18, PB22, PB19, but flow control and the clock
//! output stay off). ch32-data does not know this remap, so this is done
//! on the registers rather than with the HAL's USART driver.

use core::ptr::{read_volatile, write_volatile};

use ch32_hal as hal;

use crate::console::LINE;
use crate::log;

const BAUD: u32 = 115_200;

const USART3: usize = 0x4000_4800;
const STATR: usize = 0x00;
const DATAR: usize = 0x04;
const BRR: usize = 0x08;
const CTLR1: usize = 0x0C;
const ORE: u16 = 0x0008;
const RXNE: u16 = 0x0020;
const TXE: u16 = 0x0080;
const RE: u16 = 0x0004;
const TE: u16 = 0x0008;
const UE: u16 = 0x2000;

const AFIO_PCFR1: usize = 0x4001_0004;
const GPIOB_CFGXR: usize = 0x4001_0C00 + 0x1C;

fn rd(off: usize) -> u16 {
    unsafe { read_volatile((USART3 + off) as *const u16) }
}

fn wr(off: usize, v: u16) {
    unsafe { write_volatile((USART3 + off) as *mut u16, v) }
}

pub struct Uart {
    line: [u8; LINE],
    n: usize,
}

impl Uart {
    pub fn new() -> Self {
        hal::pac::RCC.apb2pcenr().modify(|w| {
            w.set_iopben(true);
            w.set_afioen(true);
        });
        hal::pac::RCC.apb1pcenr().modify(|w| w.set_usart3en(true));
        unsafe {
            let pcfr1 = read_volatile(AFIO_PCFR1 as *const u32);
            write_volatile(AFIO_PCFR1 as *mut u32, pcfr1 | 0b11 << 10);
            // PB16 alternate push-pull 50 MHz (0b1011), PB17 floating input
            // (R34 pulls it up)
            let cfg = read_volatile(GPIOB_CFGXR as *const u32);
            write_volatile(GPIOB_CFGXR as *mut u32, (cfg & !0xFF) | 0x4B);
        }
        let hclk = hal::rcc::clocks().hclk.0; // USART3 runs on HCLK
        wr(BRR, ((hclk + BAUD / 2) / BAUD) as u16);
        wr(CTLR1, UE | TE | RE);
        Self { line: [0; LINE], n: 0 }
    }

    /// Send the next log byte if the line is free, and take in what was
    /// typed; a finished line goes to `on_line`. Never waits.
    pub fn poll(&mut self, mut on_line: impl FnMut(&str)) {
        if rd(STATR) & TXE != 0 {
            if let Some(b) = log::pop() {
                wr(DATAR, b as u16);
            }
        }
        while rd(STATR) & (RXNE | ORE) != 0 {
            let c = rd(DATAR) as u8; // reading DATAR also clears an overrun
            match c {
                b'\r' | b'\n' => {
                    if self.n > 0 {
                        log::text("\r\n");
                        if let Ok(s) = core::str::from_utf8(&self.line[..self.n]) {
                            on_line(s);
                        }
                        self.n = 0;
                    }
                }
                0x08 | 0x7F => {
                    if self.n > 0 {
                        self.n -= 1;
                        log::text("\x08 \x08");
                    }
                }
                b' '..=b'~' if self.n < LINE - 1 => {
                    self.line[self.n] = c;
                    self.n += 1;
                    let echo = [c];
                    log::text(core::str::from_utf8(&echo).unwrap_or(""));
                }
                _ => {}
            }
        }
    }
}
