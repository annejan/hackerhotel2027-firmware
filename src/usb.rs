//! USB device for the WebUSB protocol (proto.rs). Same as `c/src/usb.c`.
//! NOT TESTED ON A CH643.
//!
//! Based on fsusb.c from ch32fun's CH32X035 USB device example (MIT,
//! CNLohr et al.), in the version from the badge team's webusb_vendor test,
//! which enumerated and ran bulk IN/OUT on a real CH32X035. The CH643's USB
//! block sits at the same address with the same registers, and WCH's CH643
//! USB device example sets up PC16/PC17 the same way, so this should carry
//! over; it is still unproven on this chip. ch32-hal has no USB driver for
//! the CH643, so this is register level, like ledpwm.rs.
//!
//! Polled (`Usb::poll`) instead of interrupt driven: while an event waits to
//! be handled the USB block answers NAK (UC_INT_BUSY), and the host simply
//! retries.
//!
//! EP0 control, EP1 bulk IN, EP2 bulk OUT. A BOS descriptor with a
//! Microsoft OS 2.0 descriptor set makes Windows bind WinUSB by itself, so
//! browsers can use WebUSB there without installing a driver.

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use crate::proto::PACKET;

/// pid.codes test PID, as in the X035 WebUSB test. A real badge needs its
/// own VID/PID (badge.team's 16D0 range, or a pid.codes allocation).
const VID: u16 = 0x1209;
const PID: u16 = 0xD035;
/// bRequest for the Microsoft OS 2.0 descriptor set
const MS_VENDOR_CODE: u8 = 0x20;

// register blocks (ch643.h)
const USB: usize = 0x4002_3400; // USBFS, AHB + 0x3400
const RCC: usize = 0x4002_1000;
const PWR: usize = 0x4000_7000;
const AFIO: usize = 0x4001_0000;
const GPIOC: usize = 0x4001_1000;

// USBFSD_TypeDef offsets
const BASE_CTRL: usize = 0x00;
const UDEV_CTRL: usize = 0x01;
const INT_EN: usize = 0x02;
const DEV_ADDR: usize = 0x03;
const INT_FG: usize = 0x06;
const INT_ST: usize = 0x07;
const RX_LEN: usize = 0x08; // 16-bit
const UEP4_1_MOD: usize = 0x0C;
const UEP2_3_MOD: usize = 0x0D;
const UEP567_MOD: usize = 0x0E;
const UEP0_DMA: usize = 0x10;
const UEP1_DMA: usize = 0x14;
const UEP2_DMA: usize = 0x18;
const UEP0_TX_LEN: usize = 0x20; // 16-bit
const UEP0_CTRL_H: usize = 0x22; // 16-bit
const UEP1_TX_LEN: usize = 0x24;
const UEP1_CTRL_H: usize = 0x26;
const UEP2_CTRL_H: usize = 0x2A;

// USB block bits (ch643_usb.h, WCH's USB library header)
const UC_DEV_PU_EN: u8 = 0x20;
const UC_INT_BUSY: u8 = 0x08;
const UC_RESET_SIE: u8 = 0x04;
const UC_CLR_ALL: u8 = 0x02;
const UC_DMA_EN: u8 = 0x01;
const UD_PD_DIS: u8 = 0x80;
const UD_PORT_EN: u8 = 0x01;
const UIE_SUSPEND: u8 = 0x04;
const UIE_TRANSFER: u8 = 0x02;
const UIE_BUS_RST: u8 = 0x01;
const UIF_TRANSFER: u8 = 0x02;
const UIF_BUS_RST: u8 = 0x01;
const UIS_TOG_OK: u8 = 0x40;
const UIS_TOKEN_MASK: u8 = 0x30;
const UIS_TOKEN_OUT: u8 = 0x00;
const UIS_TOKEN_IN: u8 = 0x20;
const UIS_TOKEN_SETUP: u8 = 0x30;
const UIS_ENDP_MASK: u8 = 0x0F;
const UDA_GP_BIT: u8 = 0x80;
const UEP1_TX_EN: u8 = 0x40;
const UEP2_RX_EN: u8 = 0x08;
const T_TOG: u16 = 1 << 6;
const T_RES_MASK: u16 = 3;
const T_RES_ACK: u16 = 0;
const T_RES_NAK: u16 = 1 << 1;
const T_RES_STALL: u16 = 3;
const R_TOG: u16 = 1 << 7;
const R_RES_MASK: u16 = 3 << 2;
const R_RES_ACK: u16 = 0;
const R_RES_NAK: u16 = 1 << 3;
const R_RES_STALL: u16 = 3 << 2;

// RCC, PWR, AFIO, GPIO registers and bits (ch643.h)
const RCC_AHBPCENR: usize = 0x14;
const RCC_APB2PCENR: usize = 0x18;
const RCC_APB1PCENR: usize = 0x1C;
const RCC_USBFS: u32 = 0x0000_1000;
const RCC_AFIOEN: u32 = 0x0000_0001;
const RCC_IOPCEN: u32 = 0x0000_0010;
const RCC_PWREN: u32 = 0x1000_0000;
const PWR_CTLR: usize = 0x00;
const PWR_CSR: usize = 0x04;
const PWR_CTLR_PLS: u32 = 0x60;
const PWR_CSR_PVDO: u32 = 0x04;
const AFIO_CTLR: usize = 0x18;
const AFIO_CTLR_UDM_PUE: u32 = 0x03;
const AFIO_CTLR_UDP_PUE: u32 = 0x0C;
const AFIO_CTLR_USB_PHY_V33: u32 = 0x40;
const AFIO_CTLR_USB_IOEN: u32 = 0x80;
/// AFIO_CTLR: pull-up on D+ (PC17)
const UDP_PUE_10K: u32 = 0x08;
const UDP_PUE_1K5: u32 = 0x0C;
const GPIO_CFGXR: usize = 0x1C;
const GPIO_BSXR: usize = 0x20;

/// The 96-bit unique ID, as on the CH32X035 (GUESS for the CH643).
const UID: usize = 0x1FFF_F7E8;

static DEVICE_DESC: [u8; 18] = [
    18, 1, // device
    0x10, 0x02, // USB 2.1: needed for the BOS descriptor
    0, 0, 0, 64, // class in the interface; EP0 64 bytes
    VID as u8, (VID >> 8) as u8, PID as u8, (PID >> 8) as u8,
    0x00, 0x01, // release 1.00
    1, 2, 3, // manufacturer, product, serial strings
    1,
];

static CONFIG_DESC: [u8; 32] = [
    9, 2, 32, 0, 1, 1, 0, 0x80, 250, // bus powered, up to 500 mA (it charges)
    9, 4, 0, 0, 2, 0xFF, 0x00, 0x00, 0, // one vendor interface, 2 endpoints
    7, 5, 0x81, 2, 64, 0, 0, // EP1 bulk IN
    7, 5, 0x02, 2, 64, 0, 0, // EP2 bulk OUT
];

const MS_SET_LEN: usize = 162;

static BOS_DESC: [u8; 33] = [
    5, 15, 33, 0, 1, // BOS, total 33 bytes, one capability
    28, 16, 5, 0, // platform capability: Microsoft OS 2.0
    0xDF, 0x60, 0xDD, 0xD8, 0x89, 0x45, 0xC7, 0x4C,
    0x9C, 0xD2, 0x65, 0x9D, 0x9E, 0x64, 0x8A, 0x9F,
    0x00, 0x00, 0x03, 0x06, // Windows 8.1 and later
    MS_SET_LEN as u8, 0, MS_VENDOR_CODE, 0,
];

/// The Microsoft OS 2.0 descriptor set: compatible ID WINUSB, and the
/// DeviceInterfaceGUIDs registry property.
static MS_SET: [u8; MS_SET_LEN] = ms_set();

const fn ms_set() -> [u8; MS_SET_LEN] {
    const HEAD: [u8; 38] = [
        10, 0, 0, 0, 0x00, 0x00, 0x03, 0x06, MS_SET_LEN as u8, 0, // set header
        20, 0, 3, 0, b'W', b'I', b'N', b'U', b'S', b'B', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // compatible ID
        132, 0, 4, 0, 7, 0, 42, 0, // registry property, REG_MULTI_SZ
    ];
    // property name and value as UTF-16LE, with their terminators
    const NAME: &[u8] = b"DeviceInterfaceGUIDs\0";
    const VALUE: &[u8] = b"{8F6D5B3C-2027-4E1A-9C55-48484837A027}\0\0";
    let mut out = [0u8; MS_SET_LEN];
    let mut i = 0;
    while i < HEAD.len() {
        out[i] = HEAD[i];
        i += 1;
    }
    let mut k = 0;
    while k < NAME.len() {
        out[i] = NAME[k];
        i += 2;
        k += 1;
    }
    out[i] = 80; // value length
    i += 2;
    k = 0;
    while k < VALUE.len() {
        out[i] = VALUE[k];
        i += 2;
        k += 1;
    }
    assert!(i == MS_SET_LEN, "Microsoft OS 2.0 descriptor set length");
    out
}

static LANG_DESC: [u8; 4] = [4, 3, 0x09, 0x04];
static STRINGS: [&[u8]; 2] = [b"badge.team", b"Hacker Hotel 2027 starfish"];

#[repr(C, align(4))]
struct Buf([u8; 64]);
static mut EP0: Buf = Buf([0; 64]);
static mut EP1: Buf = Buf([0; 64]);
static mut EP2: Buf = Buf([0; 64]);
/// Descriptors built at run time (strings, status bytes); all fit in one
/// packet. 'static so a control transfer can point at it like at the rest.
static mut DYN: [u8; 64] = [0; 64];

#[inline]
fn r8(off: usize) -> u8 {
    unsafe { read_volatile((USB + off) as *const u8) }
}
#[inline]
fn w8(off: usize, v: u8) {
    unsafe { write_volatile((USB + off) as *mut u8, v) }
}
#[inline]
fn r16(off: usize) -> u16 {
    unsafe { read_volatile((USB + off) as *const u16) }
}
#[inline]
fn w16(off: usize, v: u16) {
    unsafe { write_volatile((USB + off) as *mut u16, v) }
}
#[inline]
fn r32a(addr: usize) -> u32 {
    unsafe { read_volatile(addr as *const u32) }
}
#[inline]
fn w32a(addr: usize, v: u32) {
    unsafe { write_volatile(addr as *mut u32, v) }
}
#[inline]
fn set32a(addr: usize, bits: u32) {
    w32a(addr, r32a(addr) | bits);
}

// The buffers are only touched from `Usb`'s methods (one `Usb` exists) and
// by the USB DMA, which only writes a buffer while its endpoint is set to ACK.
fn ep0() -> &'static mut [u8; 64] {
    unsafe { &mut (*addr_of_mut!(EP0)).0 }
}
fn ep1() -> &'static mut [u8; 64] {
    unsafe { &mut (*addr_of_mut!(EP1)).0 }
}
fn ep2() -> &'static [u8; 64] {
    unsafe { &(*addr_of!(EP2)).0 }
}
fn dyn_buf() -> &'static mut [u8; 64] {
    unsafe { &mut *addr_of_mut!(DYN) }
}

pub struct Usb {
    /// rest of a control IN transfer
    tx: Option<&'static [u8]>,
    request: u8,
    addr: u8,
    config: u8,
    ep1_busy: bool,
    /// waiting for EP1 to be free
    reply: [u8; PACKET],
    reply_len: usize,
}

impl Usb {
    /// Enables the clocks, configures the block and connects the pull-up.
    pub fn new() -> Self {
        set32a(RCC + RCC_AHBPCENR, RCC_USBFS);
        set32a(RCC + RCC_APB2PCENR, RCC_AFIOEN | RCC_IOPCEN);
        set32a(RCC + RCC_APB1PCENR, RCC_PWREN);

        // VDD is the battery, or about 4.7 V from USB through the diode. Like
        // WCH's PWR_VDD_SupplyVoltage(): above PVD level 3 counts as 5 V.
        w32a(PWR + PWR_CTLR, (r32a(PWR + PWR_CTLR) & !PWR_CTLR_PLS) | 0x60);
        for i in 0..500u32 {
            core::hint::black_box(i);
        }
        let five_volt = r32a(PWR + PWR_CSR) & PWR_CSR_PVDO == 0;
        w32a(PWR + PWR_CTLR, r32a(PWR + PWR_CTLR) & !PWR_CTLR_PLS);

        // PC16 (D-) floating input, PC17 (D+) input with pull-up
        w32a(GPIOC + GPIO_CFGXR, (r32a(GPIOC + GPIO_CFGXR) & !0xFF) | 0x84);
        w32a(GPIOC + GPIO_BSXR, 1 << 1);
        let ctlr = r32a(AFIO + AFIO_CTLR) & !(AFIO_CTLR_UDP_PUE | AFIO_CTLR_UDM_PUE | AFIO_CTLR_USB_PHY_V33);
        let pull = if five_volt { UDP_PUE_10K } else { AFIO_CTLR_USB_PHY_V33 | UDP_PUE_1K5 };
        w32a(AFIO + AFIO_CTLR, ctlr | AFIO_CTLR_USB_IOEN | pull);

        w8(BASE_CTRL, UC_RESET_SIE | UC_CLR_ALL);
        w8(BASE_CTRL, 0);
        let mut usb = Self { tx: None, request: 0, addr: 0, config: 0, ep1_busy: false, reply: [0; PACKET], reply_len: 0 };
        usb.endpoints();
        w8(DEV_ADDR, 0);
        w8(INT_EN, UIE_SUSPEND | UIE_TRANSFER | UIE_BUS_RST); // flags only; no interrupt
        w8(BASE_CTRL, UC_DEV_PU_EN | UC_INT_BUSY | UC_DMA_EN);
        w8(INT_FG, 0xFF);
        w8(UDEV_CTRL, UD_PD_DIS | UD_PORT_EN);
        usb
    }

    fn endpoints(&mut self) {
        w8(UEP4_1_MOD, UEP1_TX_EN);
        w8(UEP2_3_MOD, UEP2_RX_EN);
        w8(UEP567_MOD, 0);
        // the USB DMA takes the buffers' full addresses
        unsafe {
            write_volatile((USB + UEP0_DMA) as *mut u32, addr_of!(EP0) as u32);
            write_volatile((USB + UEP1_DMA) as *mut u32, addr_of!(EP1) as u32);
            write_volatile((USB + UEP2_DMA) as *mut u32, addr_of!(EP2) as u32);
        }
        w16(UEP0_CTRL_H, R_RES_ACK | T_RES_NAK);
        w16(UEP1_CTRL_H, T_RES_NAK);
        w16(UEP2_CTRL_H, R_RES_ACK);
        self.ep1_busy = false;
        self.reply_len = 0;
    }

    /// A string descriptor from ASCII, built in DYN; 3 = the serial number,
    /// from the chip ID.
    fn string_desc(i: u8) -> &'static [u8] {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let mut serial = [0u8; 24];
        let s: &[u8] = if i == 3 {
            for (k, c) in serial.iter_mut().enumerate() {
                let word = unsafe { read_volatile((UID + 4 * (k / 8)) as *const u32) };
                *c = HEX[(word >> (28 - 4 * (k % 8)) & 0xF) as usize];
            }
            &serial
        } else {
            STRINGS[i as usize - 1]
        };
        let n = s.len().min(26);
        let d = dyn_buf();
        d[0] = (2 + 2 * n) as u8;
        d[1] = 3;
        for (k, &c) in s[..n].iter().enumerate() {
            d[2 + 2 * k] = c;
            d[3 + 2 * k] = 0;
        }
        &d[..2 + 2 * n]
    }

    /// `bytes` in DYN, for the small status replies.
    fn small(bytes: &[u8]) -> &'static [u8] {
        let d = dyn_buf();
        d[..bytes.len()].copy_from_slice(bytes);
        &d[..bytes.len()]
    }

    /// Start a control IN transfer: the first packet now, the rest on each IN.
    fn control_in(&mut self, data: &'static [u8], wlength: u16) {
        let data = &data[..data.len().min(wlength as usize)];
        let n = data.len().min(64);
        ep0()[..n].copy_from_slice(&data[..n]);
        let rest = &data[n..];
        self.tx = if rest.is_empty() { None } else { Some(rest) };
        // (no zero-length packet: none of these lengths is a multiple of 64)
        w16(UEP0_TX_LEN, n as u16);
        w16(UEP0_CTRL_H, T_TOG | T_RES_ACK); // DATA1
    }

    fn stall(&mut self) {
        w16(UEP0_CTRL_H, T_TOG | T_RES_STALL | R_TOG | R_RES_STALL);
    }

    fn setup(&mut self) {
        let b = *ep0();
        let (ty, req) = (b[0], b[1]);
        let value = u16::from_le_bytes([b[2], b[3]]);
        let index = u16::from_le_bytes([b[4], b[5]]);
        let wlength = u16::from_le_bytes([b[6], b[7]]);
        self.request = req;
        self.tx = None;
        w16(UEP0_CTRL_H, T_TOG | T_RES_NAK | R_TOG | R_RES_NAK);

        if ty & 0x60 == 0x40 {
            // vendor
            if req == MS_VENDOR_CODE && index == 7 && ty & 0x80 != 0 {
                self.control_in(&MS_SET, wlength);
            } else {
                self.stall();
            }
            return;
        }
        if ty & 0x60 != 0 {
            return self.stall();
        }
        match req {
            6 => {
                // GET_DESCRIPTOR
                let d: Option<&'static [u8]> = match value >> 8 {
                    1 => Some(&DEVICE_DESC),
                    2 => Some(&CONFIG_DESC),
                    3 => match value & 0xFF {
                        0 => Some(&LANG_DESC),
                        i @ 1..=3 => Some(Self::string_desc(i as u8)),
                        _ => None,
                    },
                    15 => Some(&BOS_DESC),
                    _ => None,
                };
                match d {
                    Some(d) => self.control_in(d, wlength),
                    None => self.stall(),
                }
                return;
            }
            5 => self.addr = (value & 0x7F) as u8, // SET_ADDRESS: takes effect after the status stage
            9 => {
                // SET_CONFIGURATION
                self.config = value as u8;
                self.endpoints();
            }
            8 => return self.control_in(Self::small(&[self.config]), wlength), // GET_CONFIGURATION
            0 => return self.control_in(Self::small(&[0, 0]), wlength), // GET_STATUS
            10 => return self.control_in(Self::small(&[0]), wlength), // GET_INTERFACE
            1 | 11 => {
                // CLEAR_FEATURE (endpoint halt), SET_INTERFACE
                if req == 1 && ty & 0x1F == 2 {
                    match index & 0x0F {
                        1 => {
                            w16(UEP1_CTRL_H, T_RES_NAK);
                            self.ep1_busy = false;
                        }
                        2 => w16(UEP2_CTRL_H, R_RES_ACK),
                        _ => {}
                    }
                }
            }
            _ => return self.stall(),
        }
        // no data stage: answer the status stage with an empty DATA1 packet
        w16(UEP0_TX_LEN, 0);
        w16(UEP0_CTRL_H, T_TOG | T_RES_ACK);
    }

    fn ep0_in(&mut self) {
        let Some(rest) = self.tx else {
            if self.request == 5 {
                // SET_ADDRESS
                w8(DEV_ADDR, (r8(DEV_ADDR) & UDA_GP_BIT) | self.addr);
            }
            w16(UEP0_CTRL_H, R_TOG | R_RES_ACK); // ready for the next request
            return;
        };
        let n = rest.len().min(64);
        ep0()[..n].copy_from_slice(&rest[..n]);
        self.tx = if rest.len() > n { Some(&rest[n..]) } else { None };
        w16(UEP0_TX_LEN, n as u16);
        w16(UEP0_CTRL_H, r16(UEP0_CTRL_H) ^ T_TOG);
    }

    /// Hand the waiting reply to EP1 once it is free; until then EP2 says
    /// NAK, so the host cannot send the next command before taking this reply.
    fn send_reply(&mut self) {
        if self.reply_len == 0 || self.ep1_busy {
            return;
        }
        ep1()[..self.reply_len].copy_from_slice(&self.reply[..self.reply_len]);
        w16(UEP1_TX_LEN, self.reply_len as u16);
        w16(UEP1_CTRL_H, (r16(UEP1_CTRL_H) & !T_RES_MASK) | T_RES_ACK);
        self.ep1_busy = true;
        self.reply_len = 0;
        w16(UEP2_CTRL_H, (r16(UEP2_CTRL_H) & !R_RES_MASK) | R_RES_ACK);
    }

    /// Service the USB block once (non-blocking). When a bulk OUT packet has
    /// arrived, call `handle(packet, &mut reply)`; it returns the reply
    /// length, which goes out on bulk IN.
    pub fn poll(&mut self, mut handle: impl FnMut(&[u8], &mut [u8; PACKET]) -> usize) {
        let fg = r8(INT_FG);
        if fg & UIF_TRANSFER != 0 {
            let st = r8(INT_ST);
            let ep = st & UIS_ENDP_MASK;
            match st & UIS_TOKEN_MASK {
                UIS_TOKEN_SETUP => self.setup(),
                UIS_TOKEN_IN => {
                    if ep == 0 {
                        self.ep0_in();
                    } else if ep == 1 {
                        w16(UEP1_CTRL_H, ((r16(UEP1_CTRL_H) & !T_RES_MASK) | T_RES_NAK) ^ T_TOG);
                        self.ep1_busy = false;
                    }
                }
                UIS_TOKEN_OUT if st & UIS_TOG_OK != 0 => {
                    if ep == 0 {
                        // status stage after an IN transfer
                        w16(UEP0_TX_LEN, 0);
                        w16(UEP0_CTRL_H, T_TOG | T_RES_ACK);
                    } else if ep == 2 {
                        let n = (r16(RX_LEN) as usize).min(64);
                        let packet = &ep2()[..n];
                        self.reply_len = handle(packet, &mut self.reply).min(PACKET);
                        w16(UEP2_CTRL_H, ((r16(UEP2_CTRL_H) & !R_RES_MASK) | R_RES_NAK) ^ R_TOG);
                    }
                }
                _ => {}
            }
        } else if fg & UIF_BUS_RST != 0 {
            self.addr = 0;
            self.config = 0;
            w8(DEV_ADDR, 0);
            self.endpoints();
        }
        if fg != 0 {
            w8(INT_FG, fg);
        }
        self.send_reply();
    }
}
