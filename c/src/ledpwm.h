#pragma once
#include <stdint.h>

void ledpwm_init(void);
/* gamma-correct the frame buffer, scale by brightness, hand it to the scanner */
void ledpwm_show(uint8_t brightness);
