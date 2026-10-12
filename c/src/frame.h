/* Frame buffer and small integer colour/maths helpers. */
#pragma once
#include <stdint.h>
#include "generated.h"

typedef struct {
    uint8_t r, g, b;
} rgb_t;

/* One brightness byte per matrix slot (COM-major), linear light. */
extern uint8_t fb[SLOTS];

/* Mono LEDs are GRID (with holes) followed by EDGE; walk them with
 * for (int i = 0; i < MONO_SPAN; i++) { slot_t s = mono_at(i); if (s == SLOT_NONE) continue; ... } */
#define MONO_SPAN (GRID_H * GRID_W + EDGE_LEDS)

static inline slot_t mono_at(int i)
{
    return i < GRID_H * GRID_W ? (&GRID[0][0])[i] : EDGE[i - GRID_H * GRID_W];
}

static inline void fb_set(slot_t s, uint8_t v)
{
    if (s != SLOT_NONE)
        fb[s] = v;
}

/* grid pixel, row 0 = top; out of range and cut-off corners are ignored */
static inline void fb_grid(int row, int col, uint8_t v)
{
    if ((unsigned)row < GRID_H && (unsigned)col < GRID_W)
        fb_set(GRID[row][col], v);
}

static inline void fb_rgb(int i, rgb_t c)
{
    fb_set(RGB[i].r, c.r);
    fb_set(RGB[i].g, c.g);
    fb_set(RGB[i].b, c.b);
}

/* a * b / 255 */
static inline uint8_t scale8(uint8_t a, uint8_t b)
{
    return (uint8_t)((a * (uint16_t)b + 255) >> 8);
}

static inline uint8_t sin8(uint8_t x)
{
    return SIN8[x];
}

static inline uint8_t radius(slot_t s) { return POLAR[s][0]; }
static inline uint8_t angle(slot_t s) { return POLAR[s][1]; }

static inline rgb_t rgb_scaled(rgb_t c, uint8_t v)
{
    return (rgb_t){scale8(c.r, v), scale8(c.g, v), scale8(c.b, v)};
}

rgb_t hsv(uint8_t h, uint8_t s, uint8_t v);

/* no libgcc for rv32 here, so no __builtin_popcount */
static inline uint32_t popcount32(uint32_t x)
{
    x = x - ((x >> 1) & 0x55555555u);
    x = (x & 0x33333333u) + ((x >> 2) & 0x33333333u);
    return (((x + (x >> 4)) & 0x0F0F0F0Fu) * 0x01010101u) >> 24;
}

/* xorshift32, good enough for sparkles and fire */
uint32_t rng_next(void);
static inline uint8_t rng_byte(void) { return (uint8_t)(rng_next() >> 24); }
