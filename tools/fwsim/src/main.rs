//! Runs the firmware's hardware-independent modules (app, menu, animations,
//! games) on the PC, with the same sources as the badge.
//!
//!   cargo run -- soak [frames]
//!       every menu item with random presses; debug build, so integer
//!       overflow and out-of-bounds panic
//!   cargo run -- record OUT FRAMES [EVENT ...]
//!       write FRAMES frames to OUT for tools/sim/fwgif.py. EVENT is
//!       FRAME:ACTION with ACTION one of press0..3, tap0..3, hold0..3,
//!       down0..3 / up0..3 (button held), charge / nocharge.
//!
//! BADGE_NAME works as for the firmware build.

use std::io::Write;

use fwsim::{app, dump, frame, Event, FRAME_BYTES};

fn soak(frames: u32) {
    let mut a = app::App::new(fwsim::nametag::Name::default());
    let mut f = frame::Frame::new();
    let mut rng = frame::Rng::new(99);
    let mut held = 0u8;
    // walk every item with ▶ taps; random play in between
    for item in 0..16 {
        for t in 0..frames {
            let x = rng.next();
            if x % 9 == 0 {
                let pad = (x >> 8) as u8 % 4;
                if pad != 2 {
                    a.handle(Event::Press(pad));
                    held |= 1 << pad;
                }
            }
            if x % 11 == 0 {
                held = 0;
            }
            if x % 4001 == 0 {
                a.handle(Event::Charging(x & 1 == 0));
            }
            if x % 997 == 0 {
                // infrared: random packets, some garbage
                let junk: Vec<u8> = (0..(x >> 3) % 30).map(|_| rng.next() as u8).collect();
                let ty = [b'H', b'R', b'T', junk.first().copied().unwrap_or(0)][(x >> 1) as usize % 4];
                a.ir_received(ty, &junk);
            }
            if x % 1013 == 0 {
                let _ = a.ir_take(); // what the IR task does
            }
            if x % 7919 == 0 {
                a.handle(Event::Selftest); // left again by a random Hold(2)
            }
            if x % 5003 == 0 {
                a.handle(Event::Hold((x >> 16) as u8 % 4));
            }
            if x % 3001 == 0 {
                a.handle(Event::Hold(2)); // menu
                a.handle(Event::Tap(3));
                a.handle(Event::Tap(2)); // select something else
            }
            f.clear();
            a.render(&mut f, held);
            let _ = t;
        }
        a.handle(Event::Hold(2));
        for _ in 0..=item {
            a.handle(Event::Tap(3));
        }
        a.handle(Event::Tap(2));
        eprintln!("item {item}: {frames} frames ok");
    }
}

fn record(path: &str, frames: u32, script: &[String]) {
    let mut events: Vec<(u32, String)> = script
        .iter()
        .map(|s| {
            let (t, a) = s.split_once(':').expect("EVENT is FRAME:ACTION");
            (t.parse().expect("frame number"), a.to_string())
        })
        .collect();
    events.sort_by_key(|e| e.0);
    let mut a = app::App::new(fwsim::nametag::Name::default());
    let mut f = frame::Frame::new();
    let mut held = 0u8;
    let mut out = Vec::new();
    let mut ev = events.iter().peekable();
    for t in 0..frames {
        let mut pressed = 0u8;
        while let Some((_, act)) = ev.next_if(|e| e.0 == t) {
            let pad = act.chars().last().and_then(|c| c.to_digit(10)).unwrap_or(0) as u8;
            match act.trim_end_matches(|c: char| c.is_ascii_digit()) {
                "press" => {
                    a.handle(Event::Press(pad));
                    pressed |= 1 << pad;
                }
                "tap" => {
                    a.handle(Event::Press(pad));
                    a.handle(Event::Tap(pad));
                    pressed |= 1 << pad;
                }
                "hold" => {
                    a.handle(Event::Press(pad));
                    a.handle(Event::Hold(pad));
                    pressed |= 1 << pad;
                }
                "down" => {
                    a.handle(Event::Press(pad));
                    held |= 1 << pad;
                    pressed |= 1 << pad;
                }
                "up" => held &= !(1 << pad),
                "charge" => a.handle(Event::Charging(true)),
                "nocharge" => a.handle(Event::Charging(false)),
                other => panic!("unknown action {other}"),
            }
        }
        f.clear();
        a.render(&mut f, held);
        let mut buf = [0u8; FRAME_BYTES];
        dump(&f, &mut buf);
        out.extend_from_slice(&buf);
        out.push(held | pressed << 4); // buttons lit in the caption
        out.push(a.brightness());
    }
    std::fs::File::create(path).unwrap().write_all(&out).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("soak") => soak(args.get(1).map_or(20_000, |n| n.parse().unwrap())),
        Some("record") => record(&args[1], args[2].parse().unwrap(), &args[3..]),
        _ => eprintln!("usage: fwsim soak [frames] | fwsim record OUT FRAMES [EVENT ...]"),
    }
}
