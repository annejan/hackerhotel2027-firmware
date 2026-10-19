/* Soak through the emulator interface (emu/emu.c, as tools/emu/emu.py
 * drives it): random presses, taps, holds, chords, charging, console
 * commands, IR packets (some garbage), factory test, sponsor reel,
 * bring-up test, fresh flashes and reboots. Under ASan / UBSan any out of
 * bounds access, overflow or undefined shift stops it.
 *   test_soak [frames]   (default 200000, about two hours of badge time) */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include "badge_emu.h"

static uint32_t seed = 7;
static uint32_t rnd(void) { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; return seed; }

int main(int argc, char **argv)
{
    long frames = argc > 1 ? atol(argv[1]) : 200000;
    uint8_t out[EMU_FRAME_MAX], pkt[64];
    uint8_t held = 0;
    int reboots = 0;
    emu_fresh();
    emu_init();
    for (long t = 0; t < frames; t++) {
        uint32_t x = rnd();
        if (x % 7 == 0) { uint8_t b = (x >> 8) % 4; emu_event(EMU_PRESS, b); held |= (uint8_t)(1u << b); }
        if (x % 11 == 0) { for (int b = 0; b < 4; b++) if (held >> b & 1) emu_event(EMU_TAP, (uint8_t)b); held = 0; }
        if (x % 997 == 0) emu_event(EMU_HOLD, (x >> 12) % 4);
        if (x % 401 == 0) emu_event(EMU_CHARGING, (x >> 9) & 1);
        if (x % 2003 == 0) { char cmd[16]; snprintf(cmd, sizeof cmd, "show %u", (x >> 5) % 24); emu_console(cmd); }
        if (x % 2503 == 0) { char cmd[16]; snprintf(cmd, sizeof cmd, "level %u", (x >> 5) % 6); emu_console(cmd); }
        if (x % 3001 == 0) emu_console("hello");
        if (x % 5003 == 0) emu_console("factory");
        if (x % 7001 == 0) emu_console("sponsors");
        if (x % 9001 == 0) emu_console("factory skip");
        if (x % 15013 == 0) emu_event(EMU_SELFTEST, 0);
        if (x % 20011 == 0) emu_event(EMU_BOOTLOADER, 0);
        if (x % 30011 == 0) { emu_fresh(); emu_init(); }
        if (x % 613 == 0) {
            int n = (x >> 4) % 30;
            for (int i = 0; i < n; i++) pkt[i] = (uint8_t)rnd();
            if (n > 0) pkt[0] = (uint8_t)"HRTx"[(x >> 3) % 4];
            emu_ir_rx(pkt, n);
        }
        if (x % 1201 == 0) {
            uint8_t in[64], reply[64];
            int n = 1 + (int)((x >> 6) % 64);
            for (int i = 0; i < n; i++) in[i] = (uint8_t)rnd();
            in[0] = (uint8_t)(1 + (x >> 3) % 7);
            int r = emu_usb(in, n, reply);
            if (r < 1 || r > 64) { printf("soak: USB reply length %d\n", r); return 1; }
        }
        (void)emu_ir_tx(pkt);
        if (emu_render(held, out)) {
            reboots++;
            emu_init();
        }
    }
    printf("soak       %ld frames, %d reboots, ok\n", frames, reboots);
    return 0;
}
