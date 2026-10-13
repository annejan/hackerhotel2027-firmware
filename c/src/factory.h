/* Factory test: the first thing a freshly flashed badge does, for the
 * volunteers at the flashing table (C firmware; same as
 * ../../src/factory.rs). Short and pass / fail, for people without a
 * serial terminal; the full bring-up test (selftest.h) is for engineers.
 * main.c starts it until it has passed once (store.h keeps that in flash);
 * `factory` on the UART console starts it too.
 *
 *   LED  every LED on: white, then the arms red, green, blue. B = all lit,
 *        A = something dark or the wrong colour
 *   ROW  one COM line, then one PWM channel at a time: exactly one moving
 *        line or set of dots, never two. B = good, A = fault
 *   KEY  press ◀, B, A and ▶ once each (automatic; 20 s)
 *   USB  USB power seen on PC3 for a second (automatic; 10 s)
 *   IR   sends test packets until one is heard: its own echo, or another
 *        badge's test packets (automatic; 10 s)
 *
 * A failed step shows its name in red; B tries that step again. A + B
 * held for 3 s goes to the bring-up test instead. Every step is logged on
 * the UART as well. */
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "events.h"

typedef enum {
    FACTORY_BUSY,
    FACTORY_PASSED, /* once, the frame it passes: keep that in flash */
    FACTORY_DONE,   /* the pass screen is over, back to the badge */
} factory_t;

void factory_start(void);
void factory_event(event_t ev, uint8_t arg);
factory_t factory_render(uint8_t held);
/* a packet of type IR_TEST was heard */
void factory_ir(void);
