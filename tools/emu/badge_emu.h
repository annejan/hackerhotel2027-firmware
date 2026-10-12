/* Interface between the badge emulator (tools/emu/emu.py) and a firmware
 * built as a shared library for the PC. Both firmwares provide it:
 *   C:    make -C c emu                        -> c/build/libhh27.so
 *   Rust: cd tools/fwsim && cargo build --lib  -> tools/fwsim/target/.../libfwsim.so
 * Only the hardware-independent part runs (app, menu, games, animations);
 * the emulator stands in for the buttons, the charger and the LEDs. */
#pragma once
#include <stdint.h>

#define EMU_ABI 1

/* events, as the button and charger code send them on the badge */
enum {
    EMU_PRESS = 0,      /* arg = button 0..3 (◀ B A ▶), on press-down */
    EMU_TAP = 1,        /* arg = button, on release of a short press */
    EMU_HOLD = 2,       /* arg = button, held 600 ms */
    EMU_CHARGING = 3,   /* arg = 1 while charging */
    EMU_BOOTLOADER = 4, /* ◀ + ▶ held 3 s */
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
/* render the next frame into out[EMU_FRAME_BYTES]; held = buttons down
 * now, bit n = button n. Returns 1 when the firmware rebooted into the USB
 * boot ROM (call emu_init to power on again). */
uint32_t emu_render(uint8_t held, uint8_t *out);
uint8_t emu_brightness(void);      /* 0..255, before gamma */

/* Optional (C firmware only for now): one packet of the badge's USB
 * protocol (c/src/proto.h), as if it came in over WebUSB. Fills out[64]
 * with the reply and returns its length. emu.py --web uses it to let the
 * browser page talk to the emulator. */
int emu_usb(const uint8_t *in, int len, uint8_t *out);

/* Optional (C firmware only for now): infrared. emu_ir_tx fills out[64]
 * with a packet the firmware wants to send (irproto.h format) and returns
 * its length, or 0. emu_ir_rx hands it a packet from another badge; it
 * goes through the badge's pulse decoder. emu.py passes them between the
 * emulators on one PC. */
int emu_ir_tx(uint8_t *out);
void emu_ir_rx(const uint8_t *in, int len);

/* Optional (C firmware only): one line for the UART console (console.h).
 * The firmware's log goes to stderr. */
void emu_console(const char *line);
