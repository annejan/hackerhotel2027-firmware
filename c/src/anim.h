#pragma once
#include <stdint.h>

#define FPS 30

/* animation phase in 1/256 turns, advancing per_s units per second */
static inline uint8_t phase(uint32_t t, uint32_t per_s)
{
    return (uint8_t)(t * per_s / FPS);
}

/* each draws frame t into fb[] (which the caller clears) */
void anim_ripple(uint32_t t);
void anim_galaxy(uint32_t t);
void anim_heartbeat(uint32_t t);
void anim_twinkle(uint32_t t);
void anim_charging(uint32_t t);
void anim_usb_boot(uint32_t t);

/* Conway's Game of Life on the grid */
void life_init(void);
void life_render(uint32_t t);

/* Doom-style fire */
void fire_reset(void);
void fire_render(void);
