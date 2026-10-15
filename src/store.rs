//! The name in flash. NOT TESTED ON HARDWARE.
//!
//! The last 256-byte page of the code flash (0x0800_F700) holds a magic
//! word, a length and the name, in word 62 the challenges' progress and in
//! its last word the stage (`app::Stage`). Saving one keeps the others. memory.x ends the code flash one page
//! early, so the firmware can never be placed there. `wchisp flash` only
//! erases the sectors the new firmware needs (plus one), so the name
//! and the stage survive an update; `wchisp erase` clears them.
//!
//! Erase and program follow WCH's FLASH_ROM_ERASE / FLASH_ROM_WRITE
//! (ch643_flash.c, 256-byte "fast" pages): unlock with the flash and the
//! fast-mode keys, erase the page, fill the page buffer word by word,
//! program, lock.

use ch32_hal as hal;
use core::ptr::{read_volatile, write_volatile};
use hal::pac::FLASH;

use crate::app::Stage;
use crate::nametag::{Name, MAX};

const PAGE: usize = 0x0800_F700;
const MAGIC: u32 = 0x4E41_4D45; // "NAME"
const PROGRESS_WORD: usize = 62;
const PROGRESS: u32 = 0x5052_0000; // "PR" in the top half, the progress below
const STAGE_WORD: usize = 63;
const TESTED: u32 = 0x5445_5354; // "TEST"
const DONE: u32 = 0x444F_4E45; // "DONE"; anything else, erased too, is new

const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;
// FLASH_CTLR bits, from ch643_flash.c
const CR_OPTER: u32 = 1 << 5;
const CR_STRT: u32 = 1 << 6;
const CR_LOCK: u32 = 1 << 7;
const CR_FLOCK: u32 = 1 << 15;
const CR_PAGE_PG: u32 = 1 << 16;
const CR_PAGE_ER: u32 = 1 << 17;
const CR_BUF_LOAD: u32 = 1 << 18;
const CR_BUF_RST: u32 = 1 << 19;
const SR_BSY: u32 = 1 << 0;

fn word(i: usize) -> u32 {
    unsafe { read_volatile((PAGE + 4 * i) as *const u32) }
}

/// The saved name, if there is one.
pub fn load() -> Option<Name> {
    let len = word(1) as usize;
    if word(0) != MAGIC || len == 0 || len > MAX {
        return None;
    }
    let mut buf = [0u8; MAX];
    for (i, b) in buf.iter_mut().enumerate().take(len) {
        *b = (word(2 + i / 4) >> (8 * (i % 4))) as u8;
    }
    Some(Name::new(&buf[..len]))
}

fn wait() {
    while FLASH.statr().read().0 & SR_BSY != 0 {}
}

fn ctlr(set: u32, clear: u32) {
    FLASH.ctlr().modify(|w| w.0 = (w.0 & !clear) | set);
}

pub fn save(name: &Name) {
    let mut page = [0xFFFF_FFFFu32; 64];
    page[0] = MAGIC;
    page[1] = name.bytes().len() as u32;
    for (i, &b) in name.bytes().iter().enumerate() {
        let w = &mut page[2 + i / 4];
        *w = (*w & !(0xFF << (8 * (i % 4)))) | (b as u32) << (8 * (i % 4));
    }
    page[PROGRESS_WORD] = word(PROGRESS_WORD);
    page[STAGE_WORD] = word(STAGE_WORD);
    program(&page);
}

fn set_word(i: usize, v: u32) {
    let mut page = [0u32; 64];
    for (k, w) in page.iter_mut().enumerate() {
        *w = word(k);
    }
    page[i] = v;
    program(&page);
}

/// The challenges' progress, packed; 0 when none is kept.
pub fn progress() -> u16 {
    let w = word(PROGRESS_WORD);
    if w & 0xFFFF_0000 == PROGRESS { w as u16 } else { 0 }
}

pub fn set_progress(packed: u16) {
    set_word(PROGRESS_WORD, PROGRESS | packed as u32);
}

pub fn stage() -> Stage {
    match word(STAGE_WORD) {
        DONE => Stage::Done,
        TESTED => Stage::Tested,
        _ => Stage::New,
    }
}

pub fn set_stage(stage: Stage) {
    set_word(
        STAGE_WORD,
        match stage {
            Stage::Done => DONE,
            Stage::Tested => TESTED,
            Stage::New => 0xFFFF_FFFF,
        },
    );
}

fn program(page: &[u32; 64]) {
    // the page is good for ~100k erases: never spend one on no change
    if page.iter().enumerate().all(|(i, &w)| word(i) == w) {
        return;
    }
    critical_section::with(|_| {
        FLASH.keyr().write(|w| w.0 = KEY1);
        FLASH.keyr().write(|w| w.0 = KEY2);
        FLASH.modekeyr().write(|w| w.0 = KEY1); // fast (page) mode
        FLASH.modekeyr().write(|w| w.0 = KEY2);

        ctlr(CR_PAGE_ER, CR_OPTER | CR_PAGE_ER);
        FLASH.addr().write(|w| w.0 = PAGE as u32);
        ctlr(CR_STRT, 0);
        wait();
        ctlr(0, CR_PAGE_ER);

        ctlr(CR_PAGE_PG, CR_OPTER | CR_PAGE_ER);
        ctlr(CR_BUF_RST, 0);
        wait();
        for (i, &w) in page.iter().enumerate() {
            unsafe { write_volatile((PAGE + 4 * i) as *mut u32, w) };
            ctlr(CR_BUF_LOAD, 0);
            wait();
        }
        FLASH.addr().write(|w| w.0 = PAGE as u32);
        ctlr(CR_STRT, 0);
        wait();
        ctlr(0, CR_PAGE_PG);

        ctlr(CR_FLOCK | CR_LOCK, 0);
    });
}
