#pragma once

/* Reboot into the CH643's factory boot ROM (USB ISP, `wchisp flash ...`). */
void reboot_to_boot_rom(void) __attribute__((noreturn));
