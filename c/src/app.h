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

/* for the USB protocol (proto.c): what a connected computer may read and set */
uint8_t app_items(void);
const char *app_label(uint8_t i);
uint8_t app_item(void);
/* show item i; false for menu-only entries (EDIT NAME, LIGHT) */
bool app_set_item(uint8_t i);
uint8_t app_levels(void);
uint8_t app_level(void);
void app_set_level(uint8_t level);
void app_name(name_t *out);
/* use this name from now on, and hand it to main.c to keep in flash */
void app_set_name(const name_t *name);
bool app_charging(void);
