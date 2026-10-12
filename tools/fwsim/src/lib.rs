//! The firmware's hardware-independent modules (app, menu, games,
//! animations), built for the PC from the same sources as the badge:
//!  * as an rlib for the `fwsim` binary (soak test, recordings)
//!  * as a cdylib for the emulator, with the C interface in
//!    ../emu/badge_emu.h
#![allow(dead_code)]

#[path = "../../../src/anim.rs"]
pub mod anim;
#[path = "../../../src/app.rs"]
pub mod app;
#[path = "../../../src/font.rs"]
pub mod font;
#[path = "../../../src/frame.rs"]
pub mod frame;
#[path = "../../../src/game.rs"]
pub mod game;
#[path = "../../../src/games/mod.rs"]
pub mod games;
#[path = "../../../src/generated.rs"]
pub mod generated;
#[path = "../../../src/nametag.rs"]
pub mod nametag;

use std::ffi::c_char;
use std::sync::Mutex;

use generated::{EDGE, GRID, GRID_H, GRID_W, NONE, RGB};

/// Same as the firmware's main.rs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Press(u8),
    Tap(u8),
    Hold(u8),
    Charging(bool),
    Bootloader,
}

pub const FRAME_BYTES: usize = GRID_H * GRID_W + EDGE.len() + RGB.len() * 3;

/// One frame as the emulator and fwgif.py want it: grid rows (0 where the
/// octagon has no LED), outline, RGB by (arm, rank).
pub fn dump(f: &frame::Frame, out: &mut [u8]) {
    let mut o = out.iter_mut();
    for row in GRID.iter() {
        for &s in row {
            *o.next().unwrap() = if s == NONE { 0 } else { f.px[s as usize] };
        }
    }
    for &s in EDGE.iter() {
        *o.next().unwrap() = f.px[s as usize];
    }
    for l in RGB.iter() {
        for s in [l.r, l.g, l.b] {
            *o.next().unwrap() = f.px[s as usize];
        }
    }
}

// ------------------------------------------------------------ emulator ---

struct Emu {
    app: app::App,
    frame: frame::Frame,
    /// frames left of the USB boot screen; main.rs does this on the badge
    boot: Option<u32>,
}

static EMU: Mutex<Option<Emu>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn emu_abi() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn emu_firmware() -> *const c_char {
    c"rust".as_ptr()
}

#[no_mangle]
pub extern "C" fn emu_fps() -> u32 {
    anim::FPS
}

#[no_mangle]
pub extern "C" fn emu_init() {
    *EMU.lock().unwrap() = Some(Emu { app: app::App::new(), frame: frame::Frame::new(), boot: None });
}

#[no_mangle]
pub extern "C" fn emu_event(kind: u8, arg: u8) {
    let mut emu = EMU.lock().unwrap();
    let Some(e) = emu.as_mut() else { return };
    let ev = match kind {
        0 => Event::Press(arg),
        1 => Event::Tap(arg),
        2 => Event::Hold(arg),
        3 => Event::Charging(arg != 0),
        4 => Event::Bootloader,
        _ => return,
    };
    if ev == Event::Bootloader {
        e.boot = e.boot.or(Some(0));
    } else {
        e.app.handle(ev);
    }
}

#[no_mangle]
pub extern "C" fn emu_brightness() -> u8 {
    EMU.lock().unwrap().as_ref().map_or(0, |e| e.app.brightness())
}

/// # Safety
/// `out` must point to FRAME_BYTES writable bytes.
#[no_mangle]
pub unsafe extern "C" fn emu_render(held: u8, out: *mut u8) -> u32 {
    let mut emu = EMU.lock().unwrap();
    let Some(e) = emu.as_mut() else { return 0 };
    e.frame.clear();
    match e.boot {
        Some(t) if t >= anim::FPS => return 1, // rebooted into the boot ROM
        Some(t) => {
            anim::usb_boot(&mut e.frame, t);
            e.boot = Some(t + 1);
        }
        None => e.app.render(&mut e.frame, held),
    }
    dump(&e.frame, std::slice::from_raw_parts_mut(out, FRAME_BYTES));
    0
}
