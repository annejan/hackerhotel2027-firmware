/* The two "badge" screens, same as ../../src/nametag.rs: your name, and
 * Hacker Hotel 2027.
 *
 * The name is set at build time:
 *   make NAME="ANNE JAN"
 * Letters, digits and ` !'*-.:?/#@` are drawn; lower case shows as upper. */
#include "anim.h"
#include "font.h"
#include "frame.h"
#include "nametag.h"

#ifndef BADGE_NAME
#define BADGE_NAME "YOUR NAME"
#endif

/* name tag: a soft sheen moves over the letters, the arms slowly cycle
 * colour, the outline sparkles */
void nametag_name(uint32_t t)
{
    draw_label(BADGE_NAME, 4, t, FPS, 255);
    /* sheen: dim the letters except where a diagonal band passes */
    int band = phase(t, 60);
    for (int r = 4; r < 4 + FONT_H; r++)
        for (int c = 0; c < GRID_W; c++) {
            slot_t s = GRID[r][c];
            if (s != SLOT_NONE && fb[s]) {
                int d = ((c + r) * 12 - band) % 256;
                if (d < 0)
                    d += 256;
                fb[s] = d < 40 ? 255 : 170;
            }
        }
    uint8_t w = phase(t, 20);
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, hsv((uint8_t)(w + RGB[i].arm * 51), 200,
                      (uint8_t)(90 + sin8((uint8_t)(w * 3 + RGB[i].rank * 30)) / 3)));
    for (int j = 0; j < EDGE_LEDS; j++)
        if (rng_byte() < 6)
            fb[EDGE[j]] = 200;
}

/* Hacker Hotel 2027: "HH" over "2027" for 4 seconds, then the full name
 * scrolls by. The outline runs marquee lights like a hotel sign; the arms
 * glow warm gold. */
void nametag_hotel(uint32_t t)
{
    static const char SCROLL[] = "HACKER HOTEL 2027";
    const uint32_t logo = 4 * FPS;
    const uint32_t scroll = (uint32_t)(text_width(SCROLL) + GRID_W) * FPS / 10;
    uint32_t k = t % (logo + scroll);
    if (k < logo) {
        draw_text("HH", 3, 1, 255);
        static const uint8_t YEAR[4] = {2, 0, 2, 7};
        for (int i = 0; i < 4; i++) {
            const uint8_t *rows = font_digit3x5(YEAR[i]);
            for (int r = 0; r < 5; r++)
                for (int c = 0; c < 3; c++)
                    if (rows[r] >> (2 - c) & 1)
                        fb_grid(9 + r, 1 + i * 4 + c, 200);
        }
    } else {
        int off = (int)((k - logo) * 10 / FPS);
        draw_text(SCROLL, GRID_W - off, 4, 255);
    }
    uint32_t step = t * 8 / FPS; /* marquee: 8 steps per second */
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = (j + step) % 3 == 0 ? 230 : 25;
    for (int i = 0; i < RGB_LEDS; i++) {
        uint8_t flicker = sin8((uint8_t)(phase(t, 90) + RGB[i].rank * 37 + RGB[i].arm * 71));
        fb_rgb(i, rgb_scaled((rgb_t){255, 140, 20}, (uint8_t)(120 + flicker / 3)));
    }
}
