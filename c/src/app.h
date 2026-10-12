#pragma once
#include <stdint.h>
#include "events.h"

void app_init(void);
/* clear fb[] and render one frame; held = pads touched right now */
void app_render(uint8_t held);
uint8_t app_brightness(void);
