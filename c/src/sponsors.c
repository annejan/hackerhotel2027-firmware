#include "anim.h"
#include "font.h"
#include "frame.h"
#include "logos.h"
#include "sponsors.h"

enum {
    ARMS_RAINBOW,  /* colours run along the arms */
    ARMS_WAVE,     /* a wave of the two colours runs out along the arms */
    ARMS_DIAGONAL, /* a rainbow sweeps diagonally across the star */
};

typedef struct {
    const char *text;    /* scrolls by; or */
    const logo_t *logo;  /* logos.h */
    uint8_t arms;
    rgb_t colour[2];     /* ARMS_WAVE: the arms alternate between these */
} item_t;

/* colours from the logos, brightened for LEDs */
#define ALLNET_BLUE {130, 178, 255}
static const item_t REEL[] = {
    {"THANKS TO", 0, ARMS_RAINBOW, {{0}}},
    {0, &LOGO_ALLNET_GLOBE, ARMS_WAVE, {ALLNET_BLUE, ALLNET_BLUE}},
    {0, &LOGO_ALLNET, ARMS_WAVE, {ALLNET_BLUE, ALLNET_BLUE}},
    {0, &LOGO_BADGETEAM, ARMS_DIAGONAL, {{0}}},
    {0, &LOGO_NICOLAI, ARMS_WAVE, {{240, 67, 0}, {70, 70, 62}}},
    {0, &LOGO_DEFEEST, ARMS_WAVE, {{255, 245, 109}, {41, 145, 255}}},
};
#define ITEMS (int)(sizeof REEL / sizeof REEL[0])

#define SPEED 20          /* columns a second, twice the name tag's */
#define LOGO_FRAMES (5 * FPS / 2) /* a logo that fits stands this long */
#define FADE (FPS / 2)

static struct {
    int item;
    uint32_t t;       /* frames into the item */
    uint32_t total_t; /* for the colours, over the whole reel */
} r;

void sponsors_start(void)
{
    r.item = 0;
    r.t = r.total_t = 0;
}

static uint32_t scroll_frames(int w)
{
    return (uint32_t)(GRID_W + w) * FPS / SPEED;
}

static bool fits(const logo_t *l)
{
    return l->w <= GRID_W && l->h <= GRID_H;
}

static uint32_t frames(const item_t *it)
{
    if (it->text)
        return scroll_frames(text_width(it->text));
    return fits(it->logo) ? LOGO_FRAMES : scroll_frames(it->logo->w);
}

/* greyscale logo with its top left corner at grid (y, x) */
static void draw_logo(const logo_t *l, int x, int y, uint8_t v)
{
    for (int row = 0; row < GRID_H; row++) {
        int lr = row - y;
        if (lr < 0 || lr >= l->h)
            continue;
        for (int col = 0; col < GRID_W; col++) {
            int lc = col - x;
            if (lc < 0 || lc >= l->w)
                continue;
            int i = lr * l->w + lc;
            int level = l->data[i / 4] >> (2 * (i % 4)) & 3;
            if (level)
                fb_grid(row, col, scale8((uint8_t)(level * 85), v));
        }
    }
}

/* one that fits stands in the middle, fading in; a wider one scrolls by,
 * and one taller than the grid pans from its top to its bottom meanwhile */
static void logo_frame(const logo_t *l, uint32_t t, uint32_t n)
{
    if (fits(l)) {
        draw_logo(l, (GRID_W - l->w) / 2, (GRID_H - l->h) / 2, t < FADE ? (uint8_t)(t * 255 / FADE) : 255);
        return;
    }
    int y = l->h <= GRID_H ? (GRID_H - l->h) / 2 : -(int)((uint32_t)(l->h - GRID_H) * t / (n - 1));
    draw_logo(l, GRID_W - (int)(t * SPEED / FPS), y, 255);
}

static void arms(const item_t *it, uint32_t total)
{
    for (int i = 0; i < RGB_LEDS; i++) {
        const rgb_led_t *led = &RGB[i];
        if (it->arms == ARMS_WAVE) {
            uint8_t wave = sin8((uint8_t)(led->rank * 36 - total * 8));
            fb_rgb(i, rgb_scaled(it->colour[led->arm % 2], (uint8_t)(90 + wave * 165 / 255)));
        } else if (it->arms == ARMS_DIAGONAL) {
            /* distance along the diagonal from bottom left to top right
             * (angle 32 of 256, clockwise from up), in 0.25 mm */
            int along = POLAR[led->r][0] * ((int)sin8((uint8_t)(POLAR[led->r][1] + 32)) - 128) / 128;
            fb_rgb(i, hsv((uint8_t)((uint32_t)(along / 2) - total * 4), 255, 200)); /* about one rainbow across the star */
        } else {
            fb_rgb(i, hsv((uint8_t)(led->arm * 51 + led->rank * 8 + total * 2), 255, 160));
        }
    }
}

bool sponsors_render(void)
{
    if (r.item >= ITEMS)
        return false;
    const item_t *it = &REEL[r.item];
    uint32_t t = r.t++, total = r.total_t++, n = frames(it);
    if (it->text)
        draw_text(it->text, GRID_W - (int)(t * SPEED / FPS), 4, 255);
    else
        logo_frame(it->logo, t, n);
    arms(it, total);
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = (uint8_t)((j + total / 2) % 8 == 0 ? 160 : 10);
    if (r.t >= n) {
        r.item++;
        r.t = 0;
    }
    return true;
}
