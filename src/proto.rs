//! The badge's USB protocol, independent of the USB driver: usb.rs on the
//! badge and the emulator (tools/fwsim) both feed it. Same as
//! `c/src/proto.c`; the browser page is `docs/webusb/index.html`.
//!
//! One command per bulk OUT packet, one reply per bulk IN packet, at most
//! 64 bytes each. The reply starts with the command byte; 0xFF = unknown
//! command or bad argument (then the command byte follows).
//!
//!   0x01 INFO                -> 01 version items levels name_max firmware('r')
//!   0x02 GET NAME            -> 02 len chars...
//!   0x03 SET NAME len chars  -> 03 len chars...   (saved to flash)
//!   0x04 GET STATE           -> 04 item level charging
//!   0x05 SET ITEM i          -> 04 item level charging
//!   0x06 SET LEVEL l         -> 04 item level charging
//!   0x07 GET LABEL i         -> 07 i len chars...

use crate::app::App;
use crate::nametag::{Name, MAX};

pub const VERSION: u8 = 1;
pub const PACKET: usize = 64;

const INFO: u8 = 1;
const GET_NAME: u8 = 2;
const SET_NAME: u8 = 3;
const GET_STATE: u8 = 4;
const SET_ITEM: u8 = 5;
const SET_LEVEL: u8 = 6;
const GET_LABEL: u8 = 7;
const ERR: u8 = 0xFF;

fn error(cmd: u8, out: &mut [u8; PACKET]) -> usize {
    out[0] = ERR;
    out[1] = cmd;
    2
}

fn state(app: &App, out: &mut [u8; PACKET]) -> usize {
    out[0] = GET_STATE;
    out[1] = app.item() as u8;
    out[2] = app.level() as u8;
    out[3] = app.charging() as u8;
    4
}

/// `len` then the bytes of `s` at `out[at..]`; returns the reply length.
fn text(out: &mut [u8; PACKET], at: usize, s: &[u8]) -> usize {
    out[at] = s.len() as u8;
    out[at + 1..at + 1 + s.len()].copy_from_slice(s);
    at + 1 + s.len()
}

/// One command in, the reply's length out (1..=PACKET).
pub fn handle(app: &mut App, input: &[u8], out: &mut [u8; PACKET]) -> usize {
    let Some(&cmd) = input.first() else {
        return error(0, out);
    };
    let arg = input.get(1).copied();
    match cmd {
        INFO => {
            out[..6].copy_from_slice(&[INFO, VERSION, App::items() as u8, App::levels() as u8, MAX as u8, b'r']);
            6
        }
        SET_NAME | GET_NAME => {
            if cmd == SET_NAME {
                let n = match arg {
                    Some(n) if (n as usize) <= MAX && input.len() >= 2 + n as usize => n as usize,
                    _ => return error(cmd, out),
                };
                app.set_name(Name::new(&input[2..2 + n]));
            }
            // reply with the name as it is now, cleaned up
            out[0] = cmd;
            text(out, 1, app.name().bytes())
        }
        SET_ITEM => match arg {
            Some(i) if app.set_item(i as usize) => state(app, out),
            _ => error(cmd, out),
        },
        SET_LEVEL => match arg {
            Some(l) if (l as usize) < App::levels() => {
                app.set_level(l as usize);
                state(app, out)
            }
            _ => error(cmd, out),
        },
        GET_STATE => state(app, out),
        GET_LABEL => match arg {
            Some(i) if (i as usize) < App::items() => {
                let s = App::label(i as usize).as_bytes();
                out[0] = GET_LABEL;
                out[1] = i;
                text(out, 2, &s[..s.len().min(PACKET - 3)])
            }
            _ => error(cmd, out),
        },
        _ => error(cmd, out),
    }
}
