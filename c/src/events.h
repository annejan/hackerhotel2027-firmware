#pragma once
#include <stdint.h>

typedef enum {
    EV_TAP,      /* short touch, arg = pad 0..3 (T1..T4) */
    EV_HOLD,     /* touch held for TOUCH_HOLD_MS, arg = pad */
    EV_CHARGING, /* arg = 1 while the battery charges */
} event_t;

/* consumed by the animation player */
void anim_event(event_t ev, uint8_t arg);
