#pragma once
#include <stdint.h>

void ledpwm_init(void);
/* gamma-correct the frame buffer and hand it to the scanner; brightness
 * (0..255) sets the intensity PWM */
void ledpwm_show(uint8_t brightness);
/* rows shown since power on (for the bring-up test) */
uint32_t ledpwm_rows(void);
