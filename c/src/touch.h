#pragma once
#include <stdint.h>

#define TOUCH_POLL_MS 10
#define TOUCH_HOLD_MS 600
/* hold T1 + T4 together this long to reboot into the USB boot ROM; neither
 * is on ADC channel 11, so it works on every silicon lot */
#define TOUCH_BOOT_A 0
#define TOUCH_BOOT_B 3
#define TOUCH_BOOT_MS 3000

void touch_init(void);
/* call every TOUCH_POLL_MS; sends EV_PRESS / EV_TAP / EV_HOLD / EV_BOOTLOADER through app_event() */
void touch_poll(void);

/* pads touched right now, bit n = pad Tn+1; updated by touch_poll() */
extern volatile uint8_t touch_held;
