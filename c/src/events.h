#pragma once
#include <stdint.h>

typedef enum {
    EV_PRESS,    /* button pressed (sent on press-down, for games), arg = button 0..3 */
    EV_TAP,      /* short press, sent on release, arg = button 0..3 (◀ B A ▶) */
    EV_HOLD,     /* button held for BUTTONS_HOLD_MS, arg = button */
    EV_CHARGING, /* arg = 1 while the battery charges */
    EV_BOOTLOADER, /* ◀ + ▶ held for BUTTONS_BOOT_MS: reboot into the USB boot ROM */
} event_t;

/* consumed by the app (app.c) */
void app_event(event_t ev, uint8_t arg);
