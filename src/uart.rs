//! USART3 on the debug header J3: pin 1 RX (PB17), pin 2 TX (PB16), pin 5
//! VCC, pin 6 GND; 115200 8N1. Same as `c/src/uart.c`: sends what log.rs
//! collected and hands typed lines to the console. Polled by the main loop;
//! input comes in on the RX interrupt into a ring buffer, because a byte
//! takes 87 us and the USART holds only one.
//!
//! USART3_RM = 11 puts TX3 / RX3 on PB16 / PB17 (CH643RM 8.3.2.1; CTS, CK
//! and RTS would land on PB18, PB22, PB19, but flow control and the clock
//! output stay off). ch32-data does not know this remap, so this is done
//! on the registers rather than with the HAL's USART driver.

use core::cell::UnsafeCell;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU8, Ordering};

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
const RXNEIE: u16 = 0x0020;

const AFIO_PCFR1: usize = 0x4001_0004;
const GPIOB_CFGXR: usize = 0x4001_0C00 + 0x1C;

fn rd(off: usize) -> u16 {
    unsafe { read_volatile((USART3 + off) as *const u16) }
}

fn wr(off: usize, v: u16) {
    unsafe { write_volatile((USART3 + off) as *mut u16, v) }
}

const RING: usize = 128; // a whole console line
struct Ring(UnsafeCell<[u8; RING]>);
unsafe impl Sync for Ring {}
// written by the interrupt only, up to RX_HEAD; read by poll() from RX_TAIL
static RX_RING: Ring = Ring(UnsafeCell::new([0; RING]));
static RX_HEAD: AtomicU8 = AtomicU8::new(0);
static RX_TAIL: AtomicU8 = AtomicU8::new(0);

#[qingke_rt::interrupt]
fn USART3() {
    while rd(STATR) & (RXNE | ORE) != 0 {
        let c = rd(DATAR) as u8; // reading DATAR also clears an overrun
        let head = RX_HEAD.load(Ordering::Relaxed);
        let next = ((head as usize + 1) % RING) as u8;
        if next != RX_TAIL.load(Ordering::Acquire) {
            unsafe { (*RX_RING.0.get())[head as usize] = c };
            RX_HEAD.store(next, Ordering::Release);
        } // full: drop it
    }
}

pub struct Uart {
    line: [u8; LINE],
    n: usize,
}

impl Uart {
    pub fn new() -> Self {
        crate::pac::RCC.apb2pcenr().modify(|w| {
            w.set_iopben(true);
            w.set_afioen(true);
        });
        crate::pac::RCC.apb1pcenr().modify(|w| w.set_usart3en(true));
        unsafe {
            let pcfr1 = read_volatile(AFIO_PCFR1 as *const u32);
            write_volatile(AFIO_PCFR1 as *mut u32, pcfr1 | 0b11 << 10);
            // PB16 alternate push-pull 50 MHz (0b1011), PB17 floating input
            // (R36 pulls it up)
            let cfg = read_volatile(GPIOB_CFGXR as *const u32);
            write_volatile(GPIOB_CFGXR as *mut u32, (cfg & !0xFF) | 0x4B);
        }
        let hclk = crate::HCLK; // USART3 runs on HCLK
        wr(BRR, ((hclk + BAUD / 2) / BAUD) as u16);
        wr(CTLR1, UE | TE | RE | RXNEIE);
        unsafe { qingke::pfic::enable_interrupt(crate::pac::Interrupt::USART3 as u8) };
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
        loop {
            let tail = RX_TAIL.load(Ordering::Relaxed);
            if tail == RX_HEAD.load(Ordering::Acquire) {
                break;
            }
            let c = unsafe { (*RX_RING.0.get())[tail as usize] };
            RX_TAIL.store(((tail as usize + 1) % RING) as u8, Ordering::Release);
            match c {
                b'\r' | b'\n' => {
                    if self.n > 0 {
                        log::text("\r\n");
                        on_line(log::ascii(&self.line[..self.n]));
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
                    log::text(crate::log::ascii(&echo));
                }
                _ => {}
            }
        }
    }
}
