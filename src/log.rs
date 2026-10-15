//! A small debug log: text goes into a ring buffer that the UART poll
//! (uart.rs) sends out on the badge, and that the PC simulator prints.
//! Never blocks; what does not fit is dropped.

use core::cell::RefCell;
use critical_section::Mutex;

const SIZE: usize = 512;

struct Ring {
    buf: [u8; SIZE],
    head: usize,
    tail: usize,
}

static LOG: Mutex<RefCell<Ring>> = Mutex::new(RefCell::new(Ring { buf: [0; SIZE], head: 0, tail: 0 }));

pub fn text(s: &str) {
    critical_section::with(|cs| {
        let mut r = LOG.borrow_ref_mut(cs);
        for &b in s.as_bytes() {
            let next = (r.head + 1) % SIZE;
            if next == r.tail {
                return; // full: drop the rest
            }
            let head = r.head;
            r.buf[head] = b;
            r.head = next;
        }
    });
}

pub fn num(mut n: u32) {
    let mut b = [0u8; 10];
    let mut i = b.len();
    loop {
        i -= 1;
        b[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    text(crate::log::ascii(&b[i..]));
}

/// `s`, then a line end.
pub fn line(s: &str) {
    text(s);
    text("\r\n");
}

/// The next byte to send out, if any.
pub fn pop() -> Option<u8> {
    critical_section::with(|cs| {
        let mut r = LOG.borrow_ref_mut(cs);
        if r.tail == r.head {
            return None;
        }
        let b = r.buf[r.tail];
        r.tail = (r.tail + 1) % SIZE;
        Some(b)
    })
}

/// Bytes the firmware itself made (font characters, digits, typed ASCII)
/// as a str, without core's UTF-8 validator: non-ASCII becomes "".
pub fn ascii(b: &[u8]) -> &str {
    if b.is_ascii() {
        unsafe { core::str::from_utf8_unchecked(b) }
    } else {
        ""
    }
}
