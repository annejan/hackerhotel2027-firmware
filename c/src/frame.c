#include "font.h"
#include "frame.h"

uint8_t fb[SLOTS];

rgb_t hsv(uint8_t h, uint8_t s, uint8_t v)
{
    if (s == 0)
        return (rgb_t){v, v, v};
    uint8_t region = h / 43;
    uint8_t rem = (uint8_t)((h - region * 43) * 6);
    uint8_t p = scale8(v, 255 - s);
    uint8_t q = scale8(v, 255 - scale8(s, rem));
    uint8_t t = scale8(v, 255 - scale8(s, 255 - rem));
    switch (region) {
    case 0: return (rgb_t){v, t, p};
    case 1: return (rgb_t){q, v, p};
    case 2: return (rgb_t){p, v, t};
    case 3: return (rgb_t){p, q, v};
    case 4: return (rgb_t){t, p, v};
    default: return (rgb_t){v, p, q};
    }
}

static uint32_t rng_state = 0x2027u * 0x5EA5u;

uint32_t rng_next(void)
{
    uint32_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    return rng_state = x;
}

uint32_t MASK[GRID_H];

void frame_init(void)
{
    for (int r = 0; r < GRID_H; r++) {
        MASK[r] = 0;
        for (int c = 0; c < GRID_W; c++)
            if (GRID[r][c] != SLOT_NONE)
                MASK[r] |= 1u << c;
    }
}

void draw_text(const char *s, int x, int row, uint8_t v)
{
    for (int k = 0; s[k]; k++) {
        int x0 = x + k * (FONT_W + 1);
        if (x0 >= GRID_W || x0 + FONT_W < 0)
            continue;
        const uint8_t *g = font_glyph(s[k]);
        for (int dc = 0; dc < FONT_W; dc++)
            for (int r = 0; r < FONT_H; r++)
                if (g[dc] >> r & 1 && x0 + dc >= 0)
                    fb_grid(row + r, x0 + dc, v);
    }
}

void draw_label(const char *s, int row, uint32_t t, uint32_t fps, uint8_t v)
{
    int w = text_width(s);
    if (w <= GRID_W) {
        draw_text(s, (GRID_W - w) / 2, row, v);
    } else {
        /* copies follow each other every `period` columns: the one coming
         * in from the right, and the one before it, still going out on the left */
        int period = w + 8;
        int x = GRID_W - (int)(t * 10 / fps) % period;
        draw_text(s, x, row, v);
        draw_text(s, x - period, row, v);
    }
}

void draw_number(uint16_t n, int row, uint8_t v)
{
    uint8_t digits[5];
    int len = 0;
    do {
        digits[len++] = n % 10;
        n /= 10;
    } while (n && len < 5);
    int c0 = (GRID_W - (len * 4 - 1)) / 2;
    for (int k = 0; k < len; k++) {
        const uint8_t *rows = font_digit3x5(digits[len - 1 - k]);
        for (int r = 0; r < 5; r++)
            for (int c = 0; c < 3; c++)
                if (rows[r] >> (2 - c) & 1)
                    fb_grid(row + r, c0 + k * 4 + c, v);
    }
}
