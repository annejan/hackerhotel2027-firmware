#pragma once
#include <stdint.h>

typedef enum {
    EV_PRESS,    /* pad touched (sent on touch-down, for games), arg = pad 0..3 */
    EV_TAP,      /* short touch, sent on release, arg = pad 0..3 (T1..T4) */
    EV_HOLD,     /* touch held for TOUCH_HOLD_MS, arg = pad */
    EV_CHARGING, /* arg = 1 while the battery charges */
    EV_BOOTLOADER, /* T1 + T4 held for TOUCH_BOOT_MS: reboot into the USB boot ROM */
} event_t;

/* consumed by the app (app.c) */
void app_event(event_t ev, uint8_t arg);
