/* Whack-a-mole, same as ../../../src/games/whack.rs: a random arm lights
 * up from the tip towards the centre, one LED of its line at a time. Stop
 * it with any button before it reaches the centre: the sooner, the more
 * points (5 with one LED lit, 2 with four). Reaching the centre ends the
 * game, and so does pressing while nothing is lit. It speeds up as you go.
 * The grid shows the points. Challenge 2 (challenge.c). */
#include <string.h>
#include "anim.h"
#include "games.h"

#define S (game_state.whack)
#define FIRST_STEP 10 /* frames per LED at the start: 1.7 s to the centre */
#define LAST_STEP 3
#define HIT_FX 12

enum { WAIT, RUN };

static void next_mole(void)
{
    S.state = WAIT;
    S.wait = (uint16_t)(FPS / 2 + rng_next() % FPS); /* 0.5 to 1.5 s */
    S.arm = (uint8_t)((S.arm + 1 + rng_next() % 4) % 5);
    S.lit = 0;
    S.t = 0;
    S.demo_at = (uint8_t)(1 + rng_next() % 3);
}

void whack_init(void)
{
    memset(&S, 0, sizeof S);
    S.step = FIRST_STEP;
    S.arm = (uint8_t)(rng_next() % 5);
    next_mole();
}

bool whack_tick(bool demo, input_t in)
{
    if (S.fx)
        S.fx--;
    bool stop = demo ? S.state == RUN && S.lit >= S.demo_at : in.pressed != 0;
    if (S.state == WAIT) {
        if (stop && !S.fx)
            return false; /* too early (a double press on a hit does not count) */
        if (S.wait && --S.wait)
            return true;
        S.state = RUN;
        S.lit = 1;
        return true;
    }
    if (stop) {
        S.score += (uint16_t)(6 - S.lit);
        S.hits++;
        if (S.hits % 3 == 0 && S.step > LAST_STEP)
            S.step--;
        S.fx = HIT_FX;
        next_mole();
        return true;
    }
    if (++S.t >= S.step) {
        S.t = 0;
        if (++S.lit >= ARM_LINE)
            return false; /* it reached the centre */
    }
    return true;
}

void whack_draw(uint32_t t)
{
    (void)t;
    draw_number(S.score, 5, 255);
    for (uint8_t a = 0; a < 5; a++)
        arm_sides(a, (rgb_t){40, 8, 0});
    if (S.state == RUN)
        for (int k = 0; k < S.lit; k++)
            arm_line(S.arm, ARM_LINE - 1 - k, k == S.lit - 1 ? (rgb_t){255, 40, 0} : (rgb_t){120, 10, 0});
    if (S.fx) /* caught: the arms flash green */
        for (int i = 0; i < RGB_LEDS; i++)
            if (RGB[i].rank >= 3)
                fb_rgb(i, rgb_scaled((rgb_t){0, 255, 60}, (uint8_t)(S.fx * 255 / HIT_FX)));
}

uint16_t whack_score(void)
{
    return S.score;
}
