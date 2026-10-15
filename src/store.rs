//! The name in flash. NOT TESTED ON HARDWARE.
//!
//! The last two 256-byte pages of the code flash (0x0800_F600 and
//! 0x0800_F700) take turns. Each holds a magic word, a length and the name,
//! in word 60 a sequence number, in word 61 a check over the rest, in word
//! 62 the challenges' progress and in its last word the stage
//! (`app::Stage`). Saving one keeps the others. A save always writes the
//! page that is *not* current, so losing power halfway (a flat battery, a
//! pulled cable) leaves a page that fails its check, and the previous one
//! still loads. The newest page that passes its check is the current one.
//!
//! memory.x ends the code flash two pages early, so the firmware can never
//! be placed there. `wchisp flash` only erases the sectors the new firmware
//! needs (plus one), so the name and the stage survive an update as long as
//! the image stays under 60 KiB (CI checks that); `wchisp erase` clears
//! them.
//!
//! Erase and program follow WCH's FLASH_ROM_ERASE / FLASH_ROM_WRITE
//! (ch643_flash.c, 256-byte "fast" pages): unlock with the flash and the
//! fast-mode keys, erase the page, fill the page buffer word by word,
//! program, lock.

use core::ptr::{read_volatile, write_volatile};
use crate::pac::FLASH;

use crate::app::Stage;
use crate::nametag::{Name, MAX};

const PAGE_A: usize = 0x0800_F600;
const PAGE_B: usize = 0x0800_F700;
const MAGIC: u32 = 0x4E41_4D45; // "NAME"
const SEQ_WORD: usize = 60;
const CHECK_WORD: usize = 61;
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
const SR_WRPRTERR: u32 = 1 << 4;
const SR_EOP: u32 = 1 << 5;

fn at(page: usize, i: usize) -> u32 {
    unsafe { read_volatile((page + 4 * i) as *const u32) }
}

/// Over every word but the check itself; an erased page does not pass.
fn check(w: impl Fn(usize) -> u32) -> u32 {
    (0..64).filter(|&i| i != CHECK_WORD).fold(0x4848_3237, |c: u32, i| c.rotate_left(5) ^ w(i)) // "HH27"
}

fn valid(page: usize) -> bool {
    at(page, 0) == MAGIC && at(page, CHECK_WORD) == check(|i| at(page, i))
}

/// The newest page that passes its check.
fn current() -> Option<usize> {
    match (valid(PAGE_A), valid(PAGE_B)) {
        (true, true) if (at(PAGE_B, SEQ_WORD).wrapping_sub(at(PAGE_A, SEQ_WORD)) as i32) > 0 => Some(PAGE_B),
        (true, _) => Some(PAGE_A),
        (_, true) => Some(PAGE_B),
        _ => None,
    }
}

/// Word i of the current page; erased (all ones) when there is none.
fn word(i: usize) -> u32 {
    current().map_or(0xFFFF_FFFF, |p| at(p, i))
}

/// The saved name, if there is one.
pub fn load() -> Option<Name> {
    let page = current()?;
    let len = at(page, 1) as usize;
    if len == 0 || len > MAX {
        return None;
    }
    let mut buf = [0u8; MAX];
    for (i, b) in buf.iter_mut().enumerate().take(len) {
        *b = (at(page, 2 + i / 4) >> (8 * (i % 4))) as u8;
    }
    Some(Name::new(&buf[..len]))
}

fn wait() {
    while FLASH.statr().read().0 & SR_BSY != 0 {}
}

fn ctlr(set: u32, clear: u32) {
    FLASH.ctlr().modify(|w| w.0 = (w.0 & !clear) | set);
}

/// false when the flash write failed.
pub fn save(name: &Name) -> bool {
    let mut page = [0xFFFF_FFFFu32; 64];
    page[0] = MAGIC;
    page[1] = name.bytes().len() as u32;
    for (i, &b) in name.bytes().iter().enumerate() {
        let w = &mut page[2 + i / 4];
        *w = (*w & !(0xFF << (8 * (i % 4)))) | (b as u32) << (8 * (i % 4));
    }
    page[PROGRESS_WORD] = word(PROGRESS_WORD);
    page[STAGE_WORD] = word(STAGE_WORD);
    program(&mut page)
}

fn set_word(i: usize, v: u32) -> bool {
    let mut page = [0xFFFF_FFFFu32; 64];
    match current() {
        Some(cur) => page.iter_mut().enumerate().for_each(|(k, w)| *w = at(cur, k)),
        None => page[1] = 0, // no name yet
    }
    page[i] = v;
    program(&mut page)
}

/// The challenges' progress, packed; 0 when none is kept.
pub fn progress() -> u16 {
    let w = word(PROGRESS_WORD);
    if w & 0xFFFF_0000 == PROGRESS { w as u16 } else { 0 }
}

pub fn set_progress(packed: u16) -> bool {
    set_word(PROGRESS_WORD, PROGRESS | packed as u32)
}

pub fn stage() -> Stage {
    match word(STAGE_WORD) {
        DONE => Stage::Done,
        TESTED => Stage::Tested,
        _ => Stage::New,
    }
}

pub fn set_stage(stage: Stage) -> bool {
    set_word(
        STAGE_WORD,
        match stage {
            Stage::Done => DONE,
            Stage::Tested => TESTED,
            Stage::New => 0xFFFF_FFFF,
        },
    )
}

/// Write the page that is not current, numbered one past it. false when
/// the controller reports an error or the page does not read back; the
/// current page then stays what it was, and the next save tries again.
fn program(page: &mut [u32; 64]) -> bool {
    let cur = current();
    // a page is good for ~100k erases: never spend one on no change
    if let Some(cur) = cur {
        let same = |(i, &w): (usize, &u32)| i == SEQ_WORD || i == CHECK_WORD || at(cur, i) == w;
        if page.iter().enumerate().all(same) {
            return true;
        }
    }
    let to = if cur == Some(PAGE_A) { PAGE_B } else { PAGE_A };
    page[0] = MAGIC;
    page[SEQ_WORD] = cur.map_or(1, |c| at(c, SEQ_WORD).wrapping_add(1));
    page[CHECK_WORD] = check(|i| page[i]);
    critical_section::with(|_| {
        FLASH.keyr().write(|w| w.0 = KEY1);
        FLASH.keyr().write(|w| w.0 = KEY2);
        FLASH.modekeyr().write(|w| w.0 = KEY1); // fast (page) mode
        FLASH.modekeyr().write(|w| w.0 = KEY2);

        ctlr(CR_PAGE_ER, CR_OPTER | CR_PAGE_ER);
        FLASH.addr().write(|w| w.0 = to as u32);
        ctlr(CR_STRT, 0);
        wait();
        ctlr(0, CR_PAGE_ER);

        ctlr(CR_PAGE_PG, CR_OPTER | CR_PAGE_ER);
        ctlr(CR_BUF_RST, 0);
        wait();
        for (i, &w) in page.iter().enumerate() {
            unsafe { write_volatile((to + 4 * i) as *mut u32, w) };
            ctlr(CR_BUF_LOAD, 0);
            wait();
        }
        FLASH.addr().write(|w| w.0 = to as u32);
        ctlr(CR_STRT, 0);
        wait();
        ctlr(0, CR_PAGE_PG);

        ctlr(CR_FLOCK | CR_LOCK, 0);
    });
    let ok = FLASH.statr().read().0 & SR_WRPRTERR == 0;
    FLASH.statr().write(|w| w.0 = SR_WRPRTERR | SR_EOP); // write 1 to clear
    ok && page.iter().enumerate().all(|(i, &w)| at(to, i) == w)
}
