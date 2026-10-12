#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "events.h"
#include "nametag.h"

/* name: the name kept in flash, or name_default() */
void app_init(const name_t *name);
/* a name the user just saved, once, for main.c to keep in flash */
bool app_saved_name(name_t *out);
/* clear fb[] and render one frame; held = pads touched right now */
void app_render(uint8_t held);
uint8_t app_brightness(void);
