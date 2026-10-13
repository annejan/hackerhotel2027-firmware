/* Infrared between badges: the IR LED D316 (anode on VCC through R37,
 * 200 ohm; cathode PB21, low = on) and the 38 kHz receiver CGQ1 (on VCC
 * now; output on PB19, low while it sees carrier). Packet format in
 * irproto.h.
 *
 * The emulator has its own ir_send() / ir_busy() (../emu/emu.c), which
 * hand whole packets to the other emulators. */
#pragma once
#include <stdbool.h>
#include <stdint.h>

void ir_init(void);
/* start sending a packet; false while one is still going out, or when
 * len is more than IR_DATA_MAX. About 0.26 s for a 24-character name. */
bool ir_send(uint8_t type, const uint8_t *data, int len);
bool ir_busy(void);
/* true: also receive what we send (the bring-up test); normally our own
 * echo is ignored */
void ir_loopback(bool on);
/* decode what came in since the last call; each good packet goes to
 * app_ir_received(). Call it often (every few ms). */
void ir_poll(void);
