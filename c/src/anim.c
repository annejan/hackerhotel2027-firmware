/* Animations, same as the Rust firmware's anim.rs. Which one runs, and
 * the controls, live in app.c. */
#include <stdbool.h>
#include <string.h>
#include "anim.h"
#include "frame.h"

void anim_ripple(uint32_t t)
{
    uint8_t w = phase(t, 171); /* one ring every 1.5 s */
    for (int i = 0; i < MONO_SPAN; i++) {
        slot_t s = mono_at(i);
        if (s == SLOT_NONE)
            continue;
        uint8_t v = sin8((uint8_t)(radius(s) * 6) - w); /* 11 mm wavelength */
        uint8_t v2 = scale8(v, v);
        fb[s] = scale8(v2, v2);
    }
    for (int i = 0; i < RGB_LEDS; i++) {
        uint8_t r = radius(RGB[i].r);
        uint8_t v = sin8((uint8_t)(r * 6) - w);
        uint8_t hue = (uint8_t)(r * 3 / 2) - w / 2;
        fb_rgb(i, hsv(hue, 255, 40 + scale8(215, scale8(v, v))));
    }
}

static inline uint8_t spiral(uint8_t r, uint8_t a)
{
    return (uint8_t)(a * 5 + r * 5 / 2);
}

void anim_galaxy(uint32_t t)
{
    uint8_t w = phase(t, 85); /* one turn every 3 s */
    for (int i = 0; i < MONO_SPAN; i++) {
        slot_t s = mono_at(i);
        if (s == SLOT_NONE)
            continue;
        uint8_t v = sin8(spiral(radius(s), angle(s)) - w);
        int fall = 255 - radius(s) * 3 / 4;
        fb[s] = scale8(scale8(scale8(v, v), v), fall < 51 ? 51 : fall);
    }
    for (int i = 0; i < RGB_LEDS; i++) {
        slot_t s = RGB[i].r;
        uint8_t v = sin8(spiral(radius(s), angle(s)) - w);
        fb_rgb(i, hsv(angle(s) + w, 255, 40 + scale8(215, scale8(v, v))));
    }
}

/* double beat every 2.2 s, travelling outward at 120 mm/s */
static uint8_t beat(uint32_t ms, uint8_t r)
{
    uint32_t x = (ms + 2200 * 4 - r * 2u) % 2200; /* 2 ms per 0.25 mm */
    uint32_t a = x > 200 ? x - 200 : 200 - x;
    uint32_t b = x > 550 ? x - 550 : 550 - x;
    uint32_t v = (a < 150 ? 255 - a * 255 / 150 : 0) + (b < 150 ? 180 - b * 180 / 150 : 0);
    return v > 255 ? 255 : (uint8_t)v;
}

void anim_heartbeat(uint32_t t)
{
    uint32_t ms = t * 1000 / FPS;
    for (int i = 0; i < MONO_SPAN; i++) {
        slot_t s = mono_at(i);
        if (s != SLOT_NONE)
            fb[s] = scale8(beat(ms, radius(s)), 230);
    }
    for (int i = 0; i < RGB_LEDS; i++) {
        uint32_t v = 15u + beat(ms, radius(RGB[i].r));
        fb_rgb(i, rgb_scaled((rgb_t){255, 13, 46}, v > 255 ? 255 : (uint8_t)v));
    }
}

void anim_twinkle(uint32_t t)
{
    for (int i = 0; i < MONO_SPAN; i++) {
        slot_t s = mono_at(i);
        if (s == SLOT_NONE)
            continue;
        /* per-LED speed and phase from a cheap hash of the slot */
        uint32_t h = s * 2654435761u;
        uint8_t v = sin8(phase(t, 40 + (h >> 28) * 12) + (uint8_t)(h >> 8));
        uint32_t o = v > 215 ? (v - 215u) * 6 : 0;
        fb[s] = o > 255 ? 255 : (uint8_t)o;
    }
    /* a comet runs up each arm every 3 s, arms staggered */
    for (int i = 0; i < RGB_LEDS; i++) {
        int pos = (uint8_t)(phase(t, 85) + RGB[i].arm * 51) * 11 * 16 / 256 - 32;
        int d = pos - RGB[i].rank * 16;
        if (d >= 0 && d < 64)
            fb_rgb(i, hsv(140 + RGB[i].arm * 25, 150 + d, 255 >> (d / 16)));
    }
}

/* battery gauge; the board has no battery voltage sense, so it only shows
 * that it charges, not how full */
void anim_charging(uint32_t t)
{
    uint32_t ms = t * 1000 / FPS % 4000;
    const int r0 = 4, r1 = 11, c0 = 2, c1 = 13;
    for (int c = c0; c <= c1; c++) {
        fb_grid(r0, c, 200);
        fb_grid(r1, c, 200);
    }
    for (int r = r0; r <= r1; r++) {
        fb_grid(r, c0, 200);
        fb_grid(r, c1, 200);
    }
    for (int r = 6; r <= 9; r++)
        fb_grid(r, c1 + 1, 200);
    int cells = ms < 3000 ? (int)(ms * 10 / 3000) + 1 : 10;
    for (int c = 0; c < cells; c++) {
        bool on = c + 1 < cells || ms >= 3000 || (ms / 150) % 2 == 0;
        for (int r = r0 + 2; r <= r1 - 2; r++)
            fb_grid(r, c0 + 1 + c, on ? 160 : 40);
    }
    uint8_t w = phase(t, 40);
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = scale8(60, sin8(w + (uint8_t)(j * 256 / EDGE_LEDS)));
    for (int i = 0; i < RGB_LEDS; i++)
        if (RGB[i].rank < cells * 7 / 10)
            fb_rgb(i, hsv((uint8_t)(cells * 85 / 10), 255, 200));
}

/* shown for a second before rebooting into the USB boot ROM: an arrow
 * moving down the grid (towards the USB port), arms filling blue */
void anim_usb_boot(uint32_t t)
{
    int shift = (int)(t * 4 / FPS); /* 4 rows per second */
    for (int r = 0; r < 9; r++)
        for (int c = 7; c <= 9; c++)
            fb_grid(r + shift, c, 220);
    static const uint8_t head[] = {7, 5, 3, 1};
    for (int i = 0; i < 4; i++)
        for (int c = 8 - head[i] / 2; c <= 8 + head[i] / 2; c++)
            fb_grid(9 + i + shift, c, 255);
    uint32_t fill = t * 7 / FPS;
    for (int i = 0; i < RGB_LEDS; i++)
        if (RGB[i].rank <= fill)
            fb_rgb(i, (rgb_t){20, 60, 255});
}

/* ------------------------------------------------------------- life --- */

/* Conway's Game of Life on the 17 x 16 grid, one bit per cell */
static struct {
    uint32_t cells[GRID_H], prev[GRID_H], seen[2];
} life;

static void life_seed(void)
{
    /* ~34% alive: (a & b) is 25%, OR (c & d & e) adds another ~9% */
    for (int r = 0; r < GRID_H; r++)
        life.cells[r] = ((rng_next() & rng_next()) | (rng_next() & rng_next() & rng_next())) & MASK[r];
    memcpy(life.prev, life.cells, sizeof life.cells);
}

void life_init(void)
{
    life.seen[0] = life.seen[1] = 0;
    life_seed();
}

static uint32_t population(void)
{
    uint32_t n = 0;
    for (int r = 0; r < GRID_H; r++)
        n += popcount32(life.cells[r]);
    return n;
}

static void life_step(void)
{
    uint32_t next[GRID_H] = {0};
    for (int r = 0; r < GRID_H; r++) {
        for (int c = 0; c < GRID_W; c++) {
            int n = 0;
            for (int dr = -1; dr <= 1; dr++)
                for (int dc = -1; dc <= 1; dc++) {
                    int rr = r + dr, cc = c + dc;
                    if ((dr || dc) && rr >= 0 && rr < GRID_H && cc >= 0 && cc < GRID_W &&
                        (life.cells[rr] >> cc & 1))
                        n++;
                }
            bool alive = life.cells[r] >> c & 1;
            if (n == 3 || (alive && n == 2))
                next[r] |= 1u << c;
        }
        next[r] &= MASK[r];
    }
    memcpy(life.prev, life.cells, sizeof life.cells);
    memcpy(life.cells, next, sizeof next);

    /* reseed when it dies out or settles into a still life / blinker */
    uint32_t hash = 0x811C9DC5u;
    for (int r = 0; r < GRID_H; r++)
        hash = (hash ^ life.cells[r]) * 0x01000193u;
    if (population() < 8 || hash == life.seen[0] || hash == life.seen[1]) {
        life_seed();
        life.seen[0] = life.seen[1] = 0;
    } else {
        life.seen[0] = life.seen[1];
        life.seen[1] = hash;
    }
}

void life_render(uint32_t t)
{
    const uint32_t gen_frames = FPS / 5; /* 5 generations per second */
    uint32_t k = t % gen_frames;
    if (k == 0 && t > 0)
        life_step();
    uint32_t births = 0;
    for (int r = 0; r < GRID_H; r++) {
        uint32_t born = life.cells[r] & ~life.prev[r];
        uint32_t died = life.prev[r] & ~life.cells[r];
        births += popcount32(born);
        for (int c = 0; c < GRID_W; c++) {
            uint8_t v = born >> c & 1            ? 255
                        : life.cells[r] >> c & 1 ? 200
                        : died >> c & 1          ? (uint8_t)(120 - 120 * k / gen_frames)
                                                 : 0;
            fb_grid(r, c, v);
        }
    }
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = 12;
    uint32_t pop = population();
    uint32_t flash = (births * 20 > 200 ? 200 : births * 20) * (gen_frames - k) / gen_frames;
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, hsv((uint8_t)(85 + pop * 2 + RGB[i].rank * 8), 255, (uint8_t)(30 + flash)));
}

/* ------------------------------------------------------------- fire --- */

/* Doom-style fire: heat rises from two rows below the grid and cools */
static uint8_t heat[GRID_H + 2][GRID_W];

void fire_reset(void)
{
    memset(heat, 0, sizeof heat);
}

void fire_render(void)
{
    const int h = GRID_H + 2;
    for (int x = 0; x < GRID_W; x++)
        heat[h - 1][x] = 128 + rng_byte() / 2;
    for (int y = 0; y < h - 1; y++)
        for (int x = 0; x < GRID_W; x++) {
            int shift = rng_byte() % 3; /* -1, 0, +1 */
            uint8_t src = heat[y + 1][(x + GRID_W + shift - 1) % GRID_W];
            int v = src - (5 + rng_byte() % 34);
            heat[y][x] = v < 0 ? 0 : (uint8_t)v;
        }
    for (int r = 0; r < GRID_H; r++)
        for (int c = 0; c < GRID_W; c++)
            fb_grid(r, c, scale8(heat[r][c], heat[r][c]));
    /* the lower half of the outline and arms glow like embers */
    for (int j = 0; j < EDGE_LEDS; j++) {
        uint8_t down = 255 - sin8(angle(EDGE[j]) + 64);
        fb[EDGE[j]] = scale8(down > 90 ? down - 90 : 0, rng_byte() / 2 + 64);
    }
    for (int i = 0; i < RGB_LEDS; i++) {
        uint8_t down = 255 - sin8(angle(RGB[i].r) + 64);
        uint8_t v = scale8(down, 100 + rng_byte() / 2);
        fb_rgb(i, rgb_scaled((rgb_t){255, (uint8_t)(40 + v / 4), 4}, v));
    }
}
