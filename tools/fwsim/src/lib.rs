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
#[path = "../../../src/editor.rs"]
pub mod editor;
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
/// The name EDIT NAME saved; kept over emu_init like the badge's flash.
static FLASH_NAME: Mutex<Option<nametag::Name>> = Mutex::new(None);

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
    let name = FLASH_NAME.lock().unwrap().unwrap_or_default();
    *EMU.lock().unwrap() = Some(Emu { app: app::App::new(name), frame: frame::Frame::new(), boot: None });
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
    if let Some(name) = e.app.saved_name() {
        *FLASH_NAME.lock().unwrap() = Some(name);
    }
    dump(&e.frame, std::slice::from_raw_parts_mut(out, FRAME_BYTES));
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use nametag::Name;

    /// Menu -> EDIT NAME, as a user would: hold T3, 14 x T4, tap T3.
    fn open_editor(a: &mut app::App) {
        a.handle(Event::Hold(2));
        for _ in 0..14 {
            a.handle(Event::Tap(3));
        }
        a.handle(Event::Tap(2));
    }

    fn frames(a: &mut app::App, n: u32, held: u8) {
        let mut f = frame::Frame::new();
        for _ in 0..n {
            f.clear();
            a.render(&mut f, held);
        }
    }

    #[test]
    fn edit_and_save() {
        let mut a = app::App::new(Name::new(b"AB"));
        open_editor(&mut a);
        frames(&mut a, 2, 0);
        a.handle(Event::Press(3)); // A -> B
        a.handle(Event::Tap(2)); // next position
        a.handle(Event::Press(0)); // B -> A
        a.handle(Event::Tap(2));
        a.handle(Event::Press(0)); // blank -> @ (wraps backwards)
        a.handle(Event::Hold(2)); // save
        assert_eq!(a.saved_name().unwrap().as_str(), "BA@");
        assert!(a.saved_name().is_none(), "handed out once");
    }

    #[test]
    fn position_wraps_after_the_name() {
        let mut a = app::App::new(Name::new(b"HI"));
        open_editor(&mut a);
        for _ in 0..3 {
            a.handle(Event::Tap(2)); // H -> I -> blank -> back to H
        }
        a.handle(Event::Press(3)); // H -> I
        a.handle(Event::Hold(2));
        assert_eq!(a.saved_name().unwrap().as_str(), "II");
    }

    #[test]
    fn held_pad_repeats() {
        let mut a = app::App::new(Name::new(b"A"));
        open_editor(&mut a);
        a.handle(Event::Press(3)); // A -> B on touch-down
        frames(&mut a, 40, 1 << 3); // then 7 repeats: frames 16, 20, ..., 40
        a.handle(Event::Hold(2));
        assert_eq!(a.saved_name().unwrap().as_str(), "I");
    }

    #[test]
    fn untouched_editor_cancels() {
        let mut a = app::App::new(Name::new(b"KEEP"));
        open_editor(&mut a);
        a.handle(Event::Press(3));
        frames(&mut a, 61 * anim::FPS, 0);
        assert!(a.saved_name().is_none());
        a.handle(Event::Hold(2)); // now the menu, not a save
        assert!(a.saved_name().is_none());
    }

    #[test]
    fn name_cleans_up() {
        assert_eq!(Name::new(b"anne jan  ").as_str(), "ANNE JAN");
        assert_eq!(Name::new(b"").as_str(), "");
        assert_eq!(Name::new(&[b'x'; 40]).as_str().len(), nametag::MAX);
    }
}
