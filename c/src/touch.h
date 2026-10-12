#pragma once

#define TOUCH_POLL_MS 10
#define TOUCH_HOLD_MS 600

void touch_init(void);
/* call every TOUCH_POLL_MS; sends EV_TAP / EV_HOLD through anim_event() */
void touch_poll(void);
