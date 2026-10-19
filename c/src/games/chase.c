/* Star Chase, same as ../../../src/games/chase.rs: a light runs round the
 * outline LEDs. Stop it (◀, B or ▶) beside the arm that is lit
 * green. Every hit speeds it up; three misses and it is over. The grid
 * shows the level. */
#include <string.h>
#include "games.h"

#define S (game_state.chase)
#define N EDGE_LEDS
#define ONE 256
#define ZONE 7 /* 1/256 turn either side of the arm tip */
#define LIVES 3
#define FX 16

static int angle_dist(uint8_t a, uint8_t b)
{
    return absi((int8_t)(uint8_t)(a - b));
}

static uint8_t tip_angle(void)
{
    for (int i = 0; i < RGB_LEDS; i++)
        if (RGB[i].arm == S.arm && RGB[i].rank == 6)
            return angle(RGB[i].r);
    return 0;
}

static bool in_zone(int j)
{
    return angle_dist(angle(EDGE[j]), tip_angle()) <= ZONE;
}

static int head(void)
{
    return wrapi(S.pos / ONE, N);
}

void chase_init(void)
{
    memset(&S, 0, sizeof S);
    S.speed = ONE * 3 / 5;
    S.arm = (uint8_t)(rng_next() % 5);
    S.lives = LIVES;
}

bool chase_tick(bool demo, input_t in)
{
    S.t++;
    if (S.fx) {
        S.fx--;
        return S.fx || S.lives;
    }
    S.pos = wrapi(S.pos + S.speed, N * ONE);
    int h = head();
    bool stop;
    if (demo)
        /* press near the end of the zone; from level 5 on, a bit late */
        stop = S.hits < 4 ? in_zone(h) && !in_zone(wrapi(h + 2, N)) : !in_zone(h) && in_zone(wrapi(h - 2, N));
    else
        stop = in.pressed & (BTN_LEFT | BTN_B | BTN_RIGHT);
    if (stop) {
        if (in_zone(h)) {
            S.hits++;
            S.speed = S.speed * 6 / 5 > ONE * 2 ? ONE * 2 : S.speed * 6 / 5; /* never jump over a zone */
            S.arm = (uint8_t)((S.arm + 1 + rng_next() % 4) % 5);
            S.fx = FX;
            S.fx_hit = true;
        } else {
            S.lives--;
            S.fx = FX;
            S.fx_hit = false;
        }
    }
    return true;
}

void chase_draw(uint32_t t)
{
    int h = head();
    for (int j = 0; j < N; j++)
        if (in_zone(j))
            fb[EDGE[j]] = 50;
    for (int k = 0; k < 6; k++)
        fb[EDGE[wrapi(h - k, N)]] = (uint8_t)(255 >> k);
    if (S.fx && S.fx_hit) {
        for (int r = 0; r < GRID_H; r++)
            for (int c = 0; c < GRID_W; c++)
                fb_grid(r, c, (uint8_t)(S.fx * 12));
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, hsv((uint8_t)(RGB[i].arm * 51 + t * 10), 255, (uint8_t)(S.fx * 15)));
    } else if (S.fx) {
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, rgb_scaled((rgb_t){255, 0, 0}, S.fx % 6 < 3 ? 230 : 0));
    } else {
        uint8_t pulse = (uint8_t)(128 + sin8((uint8_t)(t * 8)) / 2);
        arm_fill(S.arm, 7, rgb_scaled((rgb_t){20, 255, 60}, pulse));
    }
    draw_number((uint16_t)(S.hits + 1), 4, 200);
    for (int k = 0; k < S.lives; k++)
        fb_grid(12, 6 + k * 2, 120);
}

uint16_t chase_score(void)
{
    return S.hits;
}
