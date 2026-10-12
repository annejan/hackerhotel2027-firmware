//! Reboot into the CH643's factory boot ROM, which speaks WCH's ISP
//! protocol over USB (`wchisp flash ...`): a way to update the badge with
//! only a USB cable. (The SWD header J4 works too, with a WCH-LinkE.)
//!
//! Same as WCH's `SystemReset_StartMode(Start_Mode_BOOT)` followed by
//! `NVIC_SystemReset()`: unlock the flash controller, unlock the boot-mode
//! switch, set FLASH_STATR.BOOT_MODE, request a system reset.
//! Not tested on hardware.

use ch32_hal as hal;
use core::ptr::write_volatile;
use hal::pac::FLASH;

const KEY1: u32 = 0x4567_0123;
const KEY2: u32 = 0xCDEF_89AB;
const STATR_BOOT_MODE: u32 = 1 << 14;
const CTLR_LOCK: u32 = 1 << 7;

// PFIC configuration register: KEY3 + SYSRESET (core_riscv.h, NVIC_SystemReset)
const PFIC_CFGR: *mut u32 = 0xE000_E048 as *mut u32;
const PFIC_KEY3: u32 = 0xBEEF_0000;
const PFIC_SYSRESET: u32 = 1 << 7;

pub fn reboot_to_boot_rom() -> ! {
    FLASH.keyr().write(|w| w.0 = KEY1);
    FLASH.keyr().write(|w| w.0 = KEY2);
    FLASH.boot_modekeyp().write(|w| w.0 = KEY1);
    FLASH.boot_modekeyp().write(|w| w.0 = KEY2);
    FLASH.statr().modify(|w| w.0 |= STATR_BOOT_MODE);
    FLASH.ctlr().modify(|w| w.0 |= CTLR_LOCK);
    unsafe { write_volatile(PFIC_CFGR, PFIC_KEY3 | PFIC_SYSRESET) };
    loop {}
}
