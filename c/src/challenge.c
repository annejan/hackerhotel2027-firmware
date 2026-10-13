#include <string.h>
#include "anim.h"
#include "challenge.h"
#include "font.h"
#include "frame.h"

static const struct {
    game_kind_t game;
    const char *name;
    uint16_t level[CHALLENGE_LEVELS]; /* the score each level needs */
    rgb_t colour;
} CH[CHALLENGES] = {
    {GAME_MAZE, "maze", {1, 2, 3, 4, 5}, {60, 120, 255}},
    {GAME_SIMON, "simon", {4, 6, 8, 10, 12}, {255, 200, 0}},
    {GAME_WHACK, "whack-a-mole", {20, 40, 60, 80, 100}, {255, 40, 0}},
    {GAME_HILO, "higher lower", {3, 5, 7, 9, 12}, {170, 40, 255}},
    {GAME_SNAKE, "snake", {5, 10, 15, 20, 30}, {0, 255, 60}},
};

static uint8_t level[CHALLENGES];
static bool changed, victory_due;

void challenge_load(uint16_t packed)
{
    for (int c = 0; c < CHALLENGES; c++) {
        uint8_t l = packed >> (3 * c) & 7;
        level[c] = l > CHALLENGE_LEVELS ? CHALLENGE_LEVELS : l;
    }
}

uint16_t challenge_packed(void)
{
    uint16_t p = 0;
    for (int c = 0; c < CHALLENGES; c++)
        p |= (uint16_t)(level[c] << (3 * c));
    return p;
}

bool challenge_take_changed(void)
{
    bool c = changed;
    changed = false;
    return c;
}

uint8_t challenge_level(int c) { return level[c]; }
const char *challenge_name(int c) { return CH[c].name; }

int challenge_total(void)
{
    int n = 0;
    for (int c = 0; c < CHALLENGES; c++)
        n += level[c];
    return n;
}

int challenge_result(game_kind_t kind, uint16_t score)
{
    for (int c = 0; c < CHALLENGES; c++) {
        if (CH[c].game != kind)
            continue;
        uint8_t l = 0;
        while (l < CHALLENGE_LEVELS && score >= CH[c].level[l])
            l++;
        if (l <= level[c])
            return -1;
        level[c] = l;
        changed = true;
        if (challenge_total() == CHALLENGES * CHALLENGE_LEVELS)
            victory_due = true;
        return c;
    }
    return -1;
}

bool challenge_victory_due(void) { return victory_due; }
void challenge_victory_done(void) { victory_due = false; }

void challenge_arms(uint32_t t, int fresh)
{
    for (int c = 0; c < CHALLENGES; c++) {
        rgb_t col = CH[c].colour;
        for (int k = 0; k < ARM_LINE; k++) {
            uint8_t v = k >= level[c] ? 14 : c == fresh && k == level[c] - 1 && t % 12 < 6 ? 40 : 255;
            arm_line((uint8_t)c, k, rgb_scaled(col, v));
        }
        /* the two beside the line: bright, and shimmering, when it is complete */
        arm_sides((uint8_t)c, rgb_scaled(col, level[c] == CHALLENGE_LEVELS ? (uint8_t)(150 + sin8((uint8_t)(t * 6 + c * 51)) * 105 / 255) : 30));
    }
}

/* ---- the victory dance ---- */

static const char TEXT[] = "I'VE COMPLETED ALL THE CHALLENGES AND ALL I'VE GOT IS THIS LOUSY LED ANIMATION!";
#define SPEED 20  /* columns a second */
#define ROCKETS 3
#define RISE 10   /* frames up the arm */
#define BURST 26  /* frames of the burst */

typedef struct {
    uint8_t arm, hue, t; /* t = 0: free */
} rocket_t;

static struct {
    uint32_t t;
    rocket_t rocket[ROCKETS];
} v;

void victory_start(void)
{
    memset(&v, 0, sizeof v);
}

static uint8_t tip_angle(uint8_t arm)
{
    for (int i = 0; i < RGB_LEDS; i++)
        if (RGB[i].arm == arm && RGB[i].rank == 6)
            return angle(RGB[i].r);
    return 0;
}

static void fireworks(void)
{
    for (int j = 0; j < EDGE_LEDS; j++) /* a faint twinkle everywhere */
        fb[EDGE[j]] = rng_byte() < 12 ? 90 : 6;
    for (int k = 0; k < ROCKETS; k++) {
        rocket_t *r = &v.rocket[k];
        if (!r->t) {
            if (rng_byte() < 24) { /* a new one */
                r->arm = (uint8_t)(rng_next() % 5);
                r->hue = rng_byte();
                r->t = 1;
            }
            continue;
        }
        if (r->t <= RISE) { /* a spark climbs the arm, with a short trail */
            int at = (r->t - 1) * ARM_LINE / RISE;
            arm_line(r->arm, at, (rgb_t){255, 230, 180});
            if (at > 0)
                arm_line(r->arm, at - 1, (rgb_t){80, 50, 20});
        } else { /* and bursts: the arm and the outline by its tip, fading */
            uint8_t fade = (uint8_t)(255 - (r->t - RISE) * 255 / BURST);
            rgb_t c = hsv(r->hue, 230, fade);
            for (int i = 0; i < RGB_LEDS; i++)
                if (RGB[i].arm == r->arm && rng_byte() < 200)
                    fb_rgb(i, c);
            uint8_t tip = tip_angle(r->arm);
            for (int j = 0; j < EDGE_LEDS; j++) {
                int d = (int8_t)(uint8_t)(angle(EDGE[j]) - tip);
                if (d > -14 && d < 14 && rng_byte() < 150)
                    fb[EDGE[j]] = fade;
            }
        }
        if (++r->t > RISE + BURST)
            r->t = 0;
    }
}

bool victory_render(void)
{
    uint32_t t = v.t++;
    int len = (int)sizeof TEXT - 1;
    int x0 = GRID_W - (int)(t * SPEED / FPS);
    /* the letters dance: each bobs up and down, a little after the one before */
    for (int i = 0; i < len; i++) {
        int x = x0 + i * (FONT_W + 1);
        if (x <= -FONT_W || x >= GRID_W || TEXT[i] == ' ')
            continue;
        char ch[2] = {TEXT[i], 0};
        int bob = (sin8((uint8_t)(t * 12 - i * 40)) - 128) * 2 / 128;
        draw_text(ch, x, 4 + bob, 255);
    }
    fireworks();
    return x0 + len * (FONT_W + 1) > 0;
}
