/* What the hardware-independent modules call on the badge, for the tests
 * that link them without emu/emu.c: the log (kept for checks), the IR
 * sender, USB power, the LED scan counter and the boot ROM. */
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include "irproto.h"
#include "stubs.h"

char stub_log[8192];
static size_t log_len;
void log_clear(void) { log_len = 0; stub_log[0] = 0; }
void log_str(const char *s)
{
    size_t n = strlen(s);
    if (log_len + n >= sizeof stub_log) /* keep the latest text */
        log_len = 0;
    memcpy(stub_log + log_len, s, n + 1);
    log_len += n;
}
void log_u32(uint32_t n)
{
    char b[12];
    snprintf(b, sizeof b, "%u", n);
    log_str(b);
}
void log_line(const char *s)
{
    log_str(s);
    log_str("\n");
}

uint8_t stub_sent[IR_PACKET_MAX];
int stub_sent_len, stub_sends;
bool stub_ir_busy, stub_ir_loop, stub_vbus;
int stub_reboots;

bool ir_send(uint8_t type, const uint8_t *data, int len)
{
    if (stub_ir_busy)
        return false;
    stub_sent_len = ir_pack(type, data, len, stub_sent);
    stub_sends++;
    return stub_sent_len > 0;
}
bool ir_busy(void) { return stub_ir_busy; }
void ir_loopback(bool on) { stub_ir_loop = on; }
bool board_vbus(void) { return stub_vbus; }
uint32_t ledpwm_rows(void) { static uint32_t r; return r += 97; }
void reboot_to_boot_rom(void) { stub_reboots++; }
