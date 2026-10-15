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

/* smaller than libgcc's table-based __popcountsi2 */
static inline uint32_t popcount32(uint32_t x)
{
    x = x - ((x >> 1) & 0x55555555u);
    x = (x & 0x33333333u) + ((x >> 2) & 0x33333333u);
    return (((x + (x >> 4)) & 0x0F0F0F0Fu) * 0x01010101u) >> 24;
}

/* Grid LEDs per row, one bit per column (the octagon); set by frame_init() */
extern uint32_t MASK[GRID_H];
void frame_init(void);

/* is (row, col) one of the grid LEDs? */
static inline int on_grid(int r, int c)
{
    return r >= 0 && r < GRID_H && c >= 0 && c < GRID_W && (MASK[r] >> c & 1);
}

/* 5x7 text with its left edge at column x (clipped), top row `row` */
void draw_text(const char *s, int x, int row, uint8_t v);
/* text that fits is centred; longer text scrolls at 10 columns per second */
void draw_label(const char *s, int row, uint32_t t, uint32_t fps, uint8_t v);
/* number in 3x5 digits, centred, top row `row` */
void draw_number(uint16_t n, int row, uint8_t v);

/* xorshift32, good enough for sparkles and fire */
uint32_t rng_next(void);
/* mix v into the generator: the chip's unique ID, so that two badges do
 * not draw the same numbers (and send over IR at the same moments) */
void rng_seed(uint32_t v);
static inline uint8_t rng_byte(void) { return (uint8_t)(rng_next() >> 24); }
