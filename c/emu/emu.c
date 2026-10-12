/* The firmware as a shared library for the badge emulator, see
 * ../../tools/emu/badge_emu.h. Built by `make emu` with the host compiler,
 * from the hardware-independent sources only. */
#include <setjmp.h>
#include "../../tools/emu/badge_emu.h"
#include "anim.h"
#include "app.h"
#include "bootloader.h"
#include "frame.h"

#define EXPORT __attribute__((visibility("default")))

static jmp_buf boot_rom;

/* app.c shows the USB boot screen for a second, then calls this */
void reboot_to_boot_rom(void)
{
    longjmp(boot_rom, 1);
}

EXPORT uint32_t emu_abi(void) { return EMU_ABI; }
EXPORT const char *emu_firmware(void) { return "c"; }
EXPORT uint32_t emu_fps(void) { return FPS; }
EXPORT void emu_init(void) { app_init(); }
EXPORT uint8_t emu_brightness(void) { return app_brightness(); }

EXPORT void emu_event(uint8_t kind, uint8_t arg)
{
    static const event_t map[] = {EV_PRESS, EV_TAP, EV_HOLD, EV_CHARGING, EV_BOOTLOADER};
    if (kind < sizeof map / sizeof map[0])
        app_event(map[kind], arg);
}

EXPORT uint32_t emu_render(uint8_t held, uint8_t *out)
{
    if (setjmp(boot_rom))
        return 1;
    app_render(held);
    for (int r = 0; r < GRID_H; r++)
        for (int c = 0; c < GRID_W; c++)
            *out++ = GRID[r][c] == SLOT_NONE ? 0 : fb[GRID[r][c]];
    for (int j = 0; j < EDGE_LEDS; j++)
        *out++ = fb[EDGE[j]];
    for (int i = 0; i < RGB_LEDS; i++) {
        *out++ = fb[RGB[i].r];
        *out++ = fb[RGB[i].g];
        *out++ = fb[RGB[i].b];
    }
    return 0;
}
