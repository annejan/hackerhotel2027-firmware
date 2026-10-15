//! A small text console on the UART header (uart.rs feeds it lines); same
//! as `c/src/console.c`:
//!   help             list the commands
//!   info             firmware, name, what is showing, brightness
//!   name TEXT        set the name (saved to flash, like EDIT NAME)
//!   show N           show item N (as the USB SET ITEM)
//!   level N          brightness 0..4
//!   hello            send the name over IR, as B on the name tag does
//!   test             the bring-up test (selftest.rs)
//!   factory [skip]   the factory test (factory.rs); skip marks it passed
//!   sponsors         the sponsor reel (sponsors.rs)
//!   progress         the challenges' levels (challenge.rs)

use crate::app::App;
use crate::challenge::{Challenges, CHALLENGES};
use crate::log;
use crate::nametag::Name;

pub const LINE: usize = 48;

/// "word rest": the rest after `word` and spaces, when the line starts with
/// `word` followed by the end or a space.
fn arg<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(word)?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some(rest.trim_ascii_start())
}

fn number(s: &str) -> Option<usize> {
    if s.is_empty() || s.len() > 4 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn info(app: &App) {
    log::text("rust firmware, name \"");
    log::text(app.name().as_str());
    log::text("\", showing ");
    log::num(app.item() as u32);
    log::text(" ");
    log::text(App::label(app.item()));
    log::text(", level ");
    log::num(app.level() as u32);
    log::line(if app.charging() { ", charging" } else { "" });
}

pub fn line(app: &mut App, line: &str) {
    let line = line.trim_ascii_start();
    if arg(line, "help").is_some() {
        log::line("info | name TEXT | show N | level N | hello | test | factory [skip] | sponsors | progress");
        for i in 0..App::items() {
            log::text("  ");
            log::num(i as u32);
            log::text(" ");
            log::line(App::label(i));
        }
    } else if arg(line, "info").is_some() {
        info(app);
    } else if let Some(text) = arg(line, "name") {
        app.set_name(Name::new(text.as_bytes()));
        info(app);
    } else if let Some(n) = arg(line, "show").and_then(number) {
        if app.set_item(n) {
            info(app);
        } else {
            log::line("? not an item to show (see help)");
        }
    } else if let Some(n) = arg(line, "level").and_then(number).filter(|&n| n < App::levels()) {
        app.set_level(n);
        info(app);
    } else if arg(line, "test").is_some() {
        app.selftest();
    } else if let Some(rest) = arg(line, "factory") {
        if arg(rest, "skip").is_some() {
            app.factory_skip();
        } else {
            app.factory();
        }
    } else if arg(line, "sponsors").is_some() {
        app.sponsors();
    } else if arg(line, "progress").is_some() {
        let ch = app.challenges();
        for c in 0..CHALLENGES {
            log::text("  ");
            log::text(Challenges::name(c));
            log::text(" ");
            log::num(ch.level(c) as u32);
            log::line("/5");
        }
        log::text("  total ");
        log::num(ch.total() as u32);
        log::line("/25");
    } else if arg(line, "hello").is_some() {
        if !app.ir_hello() {
            log::line("ir: busy");
        }
    } else {
        log::line("? try help");
    }
}
