#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "irproto.h"

extern char stub_log[8192];   /* everything logged since log_clear() */
void log_clear(void);
extern uint8_t stub_sent[IR_PACKET_MAX]; /* the last packet ir_send() packed */
extern int stub_sent_len, stub_sends;
extern bool stub_ir_busy;   /* ir_send() refuses while set */
extern bool stub_ir_loop;   /* what ir_loopback() last asked for */
extern bool stub_vbus;      /* board_vbus() */
extern int stub_reboots;    /* reboot_to_boot_rom() calls */
