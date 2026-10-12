//! Badge-to-badge infrared: the packet format and the pulse decoder,
//! independent of the hardware (ir.rs on the badge; the PC simulator
//! passes whole packets). Same as `c/src/irproto.c`.
//!
//! On air it is NEC-style pulse-distance coding on a 38 kHz carrier:
//!   header  9 ms mark, 4.5 ms space
//!   bit     560 us mark, then 560 us space (0) or 1690 us space (1),
//!           least significant bit of each byte first
//!   end     one more 560 us mark, so the last space has an end
//! A mark is carrier on; the receiver's output is low during a mark.
//!
//! A packet is: type, length n, n data bytes, CRC-8 (polynomial 0x07) over
//! all the bytes before it.

pub const HDR_MARK_US: u32 = 9000;
pub const HDR_SPACE_US: u32 = 4500;
pub const BIT_MARK_US: u32 = 560;
pub const ZERO_SPACE_US: u32 = 560;
pub const ONE_SPACE_US: u32 = 1690;

pub const DATA_MAX: usize = 24;
pub const PACKET_MAX: usize = DATA_MAX + 3;

/// Packet types.
pub const HELLO: u8 = b'H'; // data = the sender's name; please reply
pub const REPLY: u8 = b'R'; // data = the sender's name, answering a HELLO
pub const TEST: u8 = b'T'; // the bring-up test's loopback packet

fn crc8(p: &[u8]) -> u8 {
    let mut c = 0u8;
    for &b in p {
        c ^= b;
        for _ in 0..8 {
            c = if c & 0x80 != 0 { c << 1 ^ 0x07 } else { c << 1 };
        }
    }
    c
}

/// Builds a packet in `out`; returns its length, or 0 when `data` is
/// longer than DATA_MAX.
pub fn pack(ty: u8, data: &[u8], out: &mut [u8; PACKET_MAX]) -> usize {
    if data.len() > DATA_MAX {
        return 0;
    }
    out[0] = ty;
    out[1] = data.len() as u8;
    out[2..2 + data.len()].copy_from_slice(data);
    out[2 + data.len()] = crc8(&out[..2 + data.len()]);
    data.len() + 3
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    HdrSpace,
    BitMark,
    BitSpace,
}

pub struct Decoder {
    state: State,
    bits: u8,
    byte: u8,
    n: usize,
    want: usize,
    pub buf: [u8; PACKET_MAX],
}

/// Receivers stretch marks and shorten spaces by 100 us or so, and the
/// badge's clock is an RC oscillator: allow plenty.
fn near(us: u32, want: u32) -> bool {
    us > want * 2 / 3 && us < want * 3 / 2
}

impl Decoder {
    pub const fn new() -> Self {
        Self { state: State::Idle, bits: 0, byte: 0, n: 0, want: 2, buf: [0; PACKET_MAX] }
    }

    pub fn reset(&mut self) {
        self.state = State::Idle;
    }

    /// Feed one interval as the receiver saw it: a mark (carrier) or a
    /// space, and how long it lasted. Returns the packet length once a
    /// whole packet with a good CRC has arrived (type in buf[0], length in
    /// buf[1], data from buf[2]). Anything that does not fit starts the
    /// search again.
    pub fn feed(&mut self, mark: bool, us: u32) -> usize {
        match self.state {
            State::Idle => {
                if mark && near(us, HDR_MARK_US) {
                    self.state = State::HdrSpace;
                }
                return 0;
            }
            State::HdrSpace => {
                if !mark && near(us, HDR_SPACE_US) {
                    self.state = State::BitMark;
                    self.bits = 0;
                    self.byte = 0;
                    self.n = 0;
                    self.want = 2; // until the length byte is in
                    return 0;
                }
            }
            State::BitMark => {
                if mark && us > 250 && us < 1000 {
                    self.state = State::BitSpace;
                    return 0;
                }
            }
            State::BitSpace => {
                if !mark && (250..2400).contains(&us) {
                    self.byte = self.byte >> 1 | if us >= 1000 { 0x80 } else { 0 };
                    self.state = State::BitMark;
                    self.bits += 1;
                    if self.bits < 8 {
                        return 0;
                    }
                    self.bits = 0;
                    self.buf[self.n] = self.byte;
                    self.n += 1;
                    if self.n == 2 && self.buf[1] as usize > DATA_MAX {
                        self.state = State::Idle;
                        return 0;
                    }
                    if self.n == 2 {
                        self.want = self.buf[1] as usize + 3;
                    }
                    if self.n < self.want {
                        return 0;
                    }
                    self.state = State::Idle;
                    let n = self.n;
                    return if crc8(&self.buf[..n - 1]) == self.buf[n - 1] { n } else { 0 };
                }
            }
        }
        // out of step: this interval may be the start of a new header
        self.state = State::Idle;
        self.feed(mark, us)
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
