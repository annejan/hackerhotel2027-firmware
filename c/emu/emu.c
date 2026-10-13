/* The firmware as a shared library for the badge emulator, see
 * ../../tools/emu/badge_emu.h. Built by `make emu` with the host compiler,
 * from the hardware-independent sources only. */
#include <setjmp.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include "../../tools/emu/badge_emu.h"
#include "anim.h"
#include "app.h"
#include "bootloader.h"
#include "console.h"
#include "frame.h"
#include "ir.h"
#include "irproto.h"
#include "log.h"
#include "proto.h"
#include "selftest.h"

#define EXPORT __attribute__((visibility("default")))

EXPORT void emu_ir_rx(const uint8_t *in, int len);
static void keep_saved_name(void);

static jmp_buf boot_rom;
/* the name EDIT NAME saved; kept over emu_init like the badge's flash */
static name_t flash_name;
static bool has_flash_name;
/* the stage (store.h), also kept like the flash; the emulator starts as a
 * badge that has been through the factory test and the sponsor reel */
static stage_t flash_stage = STAGE_DONE;

/* app.c shows the USB boot screen for a second, then calls this */
void reboot_to_boot_rom(void)
{
    longjmp(boot_rom, 1);
}

EXPORT uint32_t emu_abi(void) { return EMU_ABI; }
EXPORT const char *emu_firmware(void) { return "c"; }
EXPORT uint32_t emu_fps(void) { return FPS; }
EXPORT void emu_init(void)
{
    name_t name;
    if (has_flash_name)
        name = flash_name;
    else
        name_default(&name);
    app_init(&name);
    if (flash_stage == STAGE_NEW)
        app_factory();
    else if (flash_stage == STAGE_TESTED)
        app_sponsors();
}
/* as if just flashed: the factory test, then the sponsor reel */
EXPORT void emu_fresh(void) { flash_stage = STAGE_NEW; }
EXPORT uint8_t emu_brightness(void) { return app_brightness(); }

EXPORT void emu_event(uint8_t kind, uint8_t arg)
{
    static const event_t map[] = {EV_PRESS, EV_TAP, EV_HOLD, EV_CHARGING, EV_BOOTLOADER, EV_SELFTEST};
    if (kind < sizeof map / sizeof map[0])
        app_event(map[kind], arg);
}

/* hardware the bring-up test looks at */
static uint32_t rows;    /* LED rows shown: 2930 a second, as on the badge */
static bool ir_loop;     /* ir_loopback(): our packets come back */
static uint8_t ir_echo[IR_PACKET_MAX];
static int ir_echo_len;

uint32_t ledpwm_rows(void) { return rows; }
void ir_loopback(bool on) { ir_loop = on; }
bool board_vbus(void) { return app_charging(); }

EXPORT uint32_t emu_render(uint8_t held, uint8_t *out)
{
    if (setjmp(boot_rom))
        return 1;
    rows += 2930 / FPS;
    if (ir_echo_len) { /* the loopback packet, a frame later */
        int n = ir_echo_len;
        ir_echo_len = 0;
        emu_ir_rx(ir_echo, n);
    }
    app_render(held);
    keep_saved_name();
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

static void keep_saved_name(void)
{
    name_t name;
    stage_t stage;
    if (app_saved_name(&name)) { /* SET NAME: "flash" it, like main.c */
        flash_name = name;
        has_flash_name = true;
    }
    if (app_stage(&stage) && stage > flash_stage)
        flash_stage = stage;
}

EXPORT int emu_usb(const uint8_t *in, int len, uint8_t *out)
{
    int n = proto_handle(in, len, out);
    keep_saved_name();
    return n;
}

/* the debug log goes to stderr */
void log_str(const char *s) { fputs(s, stderr); }
void log_u32(uint32_t n) { fprintf(stderr, "%u", n); }
void log_line(const char *s) { fprintf(stderr, "%s\n", s); }

EXPORT void emu_console(const char *line)
{
    console_line(line);
    keep_saved_name();
}

/* infrared: a packet waits here until the emulator takes it */
static uint8_t ir_out[IR_PACKET_MAX];
static int ir_out_len;

bool ir_send(uint8_t type, const uint8_t *data, int len)
{
    if (ir_out_len)
        return false;
    ir_out_len = ir_pack(type, data, len, ir_out);
    if (ir_loop && ir_out_len) {
        memcpy(ir_echo, ir_out, (size_t)ir_out_len);
        ir_echo_len = ir_out_len;
    }
    return ir_out_len > 0;
}

bool ir_busy(void) { return ir_out_len > 0; }

EXPORT int emu_ir_tx(uint8_t *out)
{
    int n = ir_out_len;
    memcpy(out, ir_out, (size_t)n);
    ir_out_len = 0;
    return n;
}

/* a packet from another emulator: turned into marks and spaces as the
 * receiver would see them (marks a little long, spaces a little short),
 * so it goes through the badge's decoder */
EXPORT void emu_ir_rx(const uint8_t *in, int len)
{
    static ir_dec_t dec;
    int got = 0;
    ir_dec_reset(&dec);
#define FEED(mark, us) if (!got) got = ir_dec_feed(&dec, mark, us)
    FEED(true, IR_HDR_MARK_US + 100);
    FEED(false, IR_HDR_SPACE_US - 100);
    for (int k = 0; k < len * 8; k++) {
        FEED(true, IR_BIT_MARK_US + 100);
        FEED(false, (in[k / 8] >> (k % 8) & 1 ? IR_ONE_SPACE_US : IR_ZERO_SPACE_US) - 100);
    }
    FEED(true, IR_BIT_MARK_US + 100);
#undef FEED
    if (got)
        app_ir_received(dec.buf[0], dec.buf + 2, dec.buf[1]);
}
