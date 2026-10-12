/* Bring-up tests for a first board (C firmware). Start: hold A + B for
 * 3 s, or `test` on the UART console. ◀ / ▶ pick the test, hold A leaves.
 * Every step is logged on the UART (log.h), so a test can be followed
 * with a serial terminal and the board in view.
 *
 *   COM    all 24 PWM channels on, one COM line at a time (2 a second)
 *   PWM    one PWM channel on all 16 COM lines at a time
 *   LED    one LED at a time, with its D number; B pauses, tap A steps
 *   ALL    every LED on, at the current brightness (current draw)
 *   GRY    grey ramp on the grid, red / green / blue / white arms (gamma, colours)
 *   KEY    the buttons held, as letters; every button event is logged anyway
 *   SCN    LED rows per second; 2930 means the LED clock is 48 MHz HCLK
 *   IR     sends a test packet every 2 s and counts the ones it hears back
 *   PWR    USB power (PC3) and charging (CHRG)
 */
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "events.h"

void selftest_start(void);
/* false once the test mode is left (hold A) */
bool selftest_event(event_t ev, uint8_t arg);
void selftest_render(uint8_t held);
/* a packet of type IR_TEST came back */
void selftest_ir(const uint8_t *data, int len);

/* from the drivers (or the emulator) */
uint32_t ledpwm_rows(void);
void ir_loopback(bool on);
bool board_vbus(void);
