#pragma once
#include <stdint.h>

/* The four push buttons, by index (what app_event() and `held` use):
 *   0 ◀  SW3  front, lower left leg (left thumb)    PC4
 *   1 B  SW1  back, left arm (left finger)          PB20
 *   2 A  SW2  back, right arm (right finger)        PC5
 *   3 ▶  SW4  front, lower right leg (right thumb)  PB18
 * Active low, with 10k pull-ups to VCC on the board. */
#define BUTTONS_POLL_MS 10
#define BUTTONS_HOLD_MS 600
/* hold ◀ + ▶ together this long to reboot into the USB boot ROM; A + B
 * as long for the bring-up test */
#define BUTTONS_BOOT_A 0
#define BUTTONS_BOOT_B 3
#define BUTTONS_TEST_A 2
#define BUTTONS_TEST_B 1
#define BUTTONS_BOOT_MS 3000

void buttons_init(void);
/* call every BUTTONS_POLL_MS; sends EV_PRESS / EV_TAP / EV_HOLD / EV_BOOTLOADER /
 * EV_SELFTEST through app_event() */
void buttons_poll(void);

/* buttons down right now, bit n = button n; updated by buttons_poll() */
extern volatile uint8_t buttons_held;
