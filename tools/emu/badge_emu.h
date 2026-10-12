/* Interface between the badge emulator (tools/emu/emu.py) and a firmware
 * built as a shared library for the PC. Both firmwares provide it:
 *   C:    make -C c emu                        -> c/build/libhh27.so
 *   Rust: cd tools/fwsim && cargo build --lib  -> tools/fwsim/target/.../libfwsim.so
 * Only the hardware-independent part runs (app, menu, games, animations);
 * the emulator stands in for the touch pads, the charger and the LEDs. */
#pragma once
#include <stdint.h>

#define EMU_ABI 1

/* events, as the touch and charger code send them on the badge */
enum {
    EMU_PRESS = 0,      /* arg = pad 0..3 (T1..T4), on touch-down */
    EMU_TAP = 1,        /* arg = pad, on release of a short touch */
    EMU_HOLD = 2,       /* arg = pad, touch held 600 ms */
    EMU_CHARGING = 3,   /* arg = 1 while charging */
    EMU_BOOTLOADER = 4, /* T1 + T4 held 3 s */
};

/* one frame: grid (16 rows x 17 columns, 0 where the octagon has no LED),
 * outline (71, clockwise from the top), RGB (35 x r,g,b, arm 0..4 then rank
 * 0..6 from the centre); linear brightness 0..255 before gamma */
#define EMU_FRAME_BYTES (16 * 17 + 71 + 35 * 3)

uint32_t emu_abi(void);            /* EMU_ABI */
const char *emu_firmware(void);    /* "c" or "rust" */
uint32_t emu_fps(void);
void emu_init(void);               /* power on; also after a reboot */
void emu_event(uint8_t kind, uint8_t arg);
/* render the next frame into out[EMU_FRAME_BYTES]; held = pads touched
 * now, bit n = pad Tn+1. Returns 1 when the firmware rebooted into the USB
 * boot ROM (call emu_init to power on again). */
uint32_t emu_render(uint8_t held, uint8_t *out);
uint8_t emu_brightness(void);      /* 0..255, before gamma */
