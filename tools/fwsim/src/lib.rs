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
#[path = "../../../src/proto.rs"]
pub mod proto;

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

/// One packet of the badge's USB protocol, as if it came in over WebUSB;
/// fills out[64] with the reply and returns its length.
///
/// # Safety
/// `input` must point to `len` readable bytes, `out` to 64 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn emu_usb(input: *const u8, len: i32, out: *mut u8) -> i32 {
    let mut emu = EMU.lock().unwrap();
    let Some(e) = emu.as_mut() else { return 0 };
    let input = if input.is_null() || len <= 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(input, (len as usize).min(proto::PACKET))
    };
    let mut reply = [0u8; proto::PACKET];
    let n = proto::handle(&mut e.app, input, &mut reply);
    std::ptr::copy_nonoverlapping(reply.as_ptr(), out, n);
    // SET NAME: "flash" it, as main.rs does
    if let Some(name) = e.app.saved_name() {
        *FLASH_NAME.lock().unwrap() = Some(name);
    }
    n as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use nametag::Name;

    /// Menu -> EDIT NAME, as a user would: hold A, 14 x ▶, tap A.
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
    fn held_button_repeats() {
        let mut a = app::App::new(Name::new(b"A"));
        open_editor(&mut a);
        a.handle(Event::Press(3)); // A -> B on press-down
        frames(&mut a, 40, 1 << 3); // then 7 repeats: frames 16, 20, ..., 40
        a.handle(Event::Hold(2));
        assert_eq!(a.saved_name().unwrap().as_str(), "I");
    }

    #[test]
    fn idle_editor_cancels() {
        let mut a = app::App::new(Name::new(b"KEEP"));
        open_editor(&mut a);
        a.handle(Event::Press(3));
        frames(&mut a, 61 * anim::FPS, 0);
        assert!(a.saved_name().is_none());
        a.handle(Event::Hold(2)); // now the menu, not a save
        assert!(a.saved_name().is_none());
    }

    /// A long label scrolls one column per step, also when the scroll
    /// period wraps: no copy may vanish while it is still in view.
    #[test]
    fn label_scrolls_smoothly() {
        use generated::{GRID, GRID_H, GRID_W, NONE};
        let s = "HACKER HOTEL"; // 71 columns: scrolls
        let fps = 10; // one column per frame
        let grid = |t| {
            let mut f = frame::Frame::new();
            frame::draw_label(&mut f, s, 4, t, fps, 255);
            let mut g = [[0u8; GRID_W]; GRID_H];
            for r in 0..GRID_H {
                for c in 0..GRID_W {
                    if GRID[r][c] != NONE {
                        g[r][c] = f.px[GRID[r][c] as usize];
                    }
                }
            }
            g
        };
        let period = font::text_width(s) as u32 + 8;
        for t in 0..2 * period {
            let (a, b) = (grid(t), grid(t + 1));
            for r in 0..GRID_H {
                for c in 0..GRID_W - 1 {
                    if GRID[r][c] != NONE && GRID[r][c + 1] != NONE {
                        assert_eq!(b[r][c], a[r][c + 1], "jump at t={t}, row {r}, col {c}");
                    }
                }
            }
        }
    }

    fn usb(a: &mut app::App, input: &[u8]) -> Vec<u8> {
        let mut out = [0u8; proto::PACKET];
        let n = proto::handle(a, input, &mut out);
        assert!((1..=proto::PACKET).contains(&n), "reply length {n} for {input:?}");
        out[..n].to_vec()
    }

    #[test]
    fn usb_protocol() {
        let mut a = app::App::new(Name::new(b"ANNE JAN"));
        assert_eq!(usb(&mut a, &[1]), [1, proto::VERSION, 16, 5, nametag::MAX as u8, b'r']);
        assert_eq!(usb(&mut a, &[2]), b"\x02\x08ANNE JAN");
        // SET NAME cleans up, replies with the clean name, and saves it
        assert_eq!(usb(&mut a, b"\x03\x0ehacker  hotel "), b"\x03\x0dHACKER  HOTEL");
        assert_eq!(a.saved_name().unwrap().as_str(), "HACKER  HOTEL");
        assert_eq!(usb(&mut a, &[2]), b"\x02\x0dHACKER  HOTEL");
        assert_eq!(usb(&mut a, &[3, 25]), [0xFF, 3], "longer than MAX");
        assert_eq!(usb(&mut a, &[3, 5, b'A']), [0xFF, 3], "shorter than its length");
        assert_eq!(usb(&mut a, &[3]), [0xFF, 3]);
        // state, items, levels
        assert_eq!(usb(&mut a, &[4]), [4, 0, 2, 0]);
        assert_eq!(usb(&mut a, &[5, 9]), [4, 9, 2, 0]);
        assert_eq!(usb(&mut a, &[5, 14]), [0xFF, 5], "EDIT NAME is menu only");
        assert_eq!(usb(&mut a, &[5, 15]), [0xFF, 5], "LIGHT is menu only");
        assert_eq!(usb(&mut a, &[5, 16]), [0xFF, 5]);
        assert_eq!(usb(&mut a, &[5]), [0xFF, 5]);
        assert_eq!(usb(&mut a, &[6, 4]), [4, 9, 4, 0]);
        assert_eq!(usb(&mut a, &[6, 5]), [0xFF, 6]);
        a.handle(Event::Charging(true));
        assert_eq!(usb(&mut a, &[4]), [4, 9, 4, 1]);
        // labels
        assert_eq!(usb(&mut a, &[7, 13]), b"\x07\x0d\x0aSTAR CHASE");
        assert_eq!(usb(&mut a, &[7, 16]), [0xFF, 7]);
        // unknown command, empty packet
        assert_eq!(usb(&mut a, &[0x42, 1, 2]), [0xFF, 0x42]);
        assert_eq!(usb(&mut a, &[]), [0xFF, 0]);
    }

    #[test]
    fn usb_protocol_fuzz() {
        let mut a = app::App::new(Name::new(b"FUZZ"));
        let mut rng = frame::Rng::new(0x1209_D035);
        let mut f = frame::Frame::new();
        for i in 0..200_000u32 {
            let len = (rng.next() % 65) as usize;
            let mut p = [0u8; 64];
            for b in p.iter_mut().take(len) {
                *b = rng.byte();
            }
            // mostly valid command bytes, so the arguments get exercised
            if len > 0 && i % 4 != 0 {
                p[0] = 1 + (p[0] % 7);
            }
            usb(&mut a, &p[..len]);
            if i % 64 == 0 {
                f.clear();
                a.render(&mut f, 0);
            }
        }
    }

    #[test]
    fn name_cleans_up() {
        assert_eq!(Name::new(b"anne jan  ").as_str(), "ANNE JAN");
        assert_eq!(Name::new(b"").as_str(), "");
        assert_eq!(Name::new(&[b'x'; 40]).as_str().len(), nametag::MAX);
    }
}
