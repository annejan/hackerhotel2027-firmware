/* Simon says, same as ../../../src/games/simon.rs: the badge plays a
 * sequence on the four legs that have a button (◀ lower left, ▶ lower
 * right, A and B on the back of the upper arms), and you press it back.
 * Every round adds one. The top arm shows how far you are, and flashes
 * green when you got it all. A wrong button, or 5 s without one, ends it.
 * The grid shows the length. Challenge 1 (challenge.c). */
#include <string.h>
#include "anim.h"
#include "games.h"

#define S (game_state.simon)
#define GAP 5
#define GOOD_FRAMES 18
#define PRESS_FRAMES 8
#define IDLE_MAX (5 * FPS)
#define NONE 0xFF

enum { SHOW, INPUT, GOOD };

/* classic Simon colours, by the arm (BUTTON_ARM): 1 yellow, 2 red, 3 blue, 4 green */
static const rgb_t COLOUR[5] = {{255, 255, 255}, {255, 200, 0}, {255, 20, 0}, {0, 60, 255}, {0, 255, 40}};

static uint8_t on_frames(void)
{
    return (uint8_t)(S.len < 16 ? 14 - S.len / 2 : 6);
}

static void add_step(void)
{
    if (S.len < SIMON_MAX)
        S.seq[S.len++] = (uint8_t)(rng_next() % 4);
    S.state = SHOW;
    S.pos = 0;
    S.t = 0;
}

void simon_init(void)
{
    memset(&S, 0, sizeof S);
    S.lit = NONE;
    S.demo_slip = (uint8_t)(3 + rng_next() % 8);
    add_step();
}

bool simon_tick(bool demo, input_t in)
{
    S.t++;
    if (S.lit_t && --S.lit_t == 0)
        S.lit = NONE;
    switch (S.state) {
    case SHOW: {
        uint8_t step = (uint8_t)(on_frames() + GAP);
        if (S.t == FPS / 2 + (uint16_t)S.pos * step + 1 && S.pos < S.len) {
            S.lit = S.seq[S.pos++];
            S.lit_t = on_frames();
        } else if (S.pos == S.len && !S.lit_t && S.t > FPS / 2) {
            S.state = INPUT;
            S.pos = 0;
            S.idle = 0;
        }
        return true;
    }
    case INPUT: {
        int b = -1;
        if (demo) {
            if (++S.idle >= 12)
                b = S.len == S.demo_slip && S.pos == S.len - 1 ? (S.seq[S.pos] + 1) % 4 : S.seq[S.pos];
        } else {
            for (int k = 0; k < 4 && b < 0; k++)
                if (in.pressed >> k & 1)
                    b = k;
            if (b < 0 && ++S.idle >= IDLE_MAX)
                return false;
        }
        if (b < 0)
            return true;
        S.idle = 0;
        S.lit = (uint8_t)b;
        S.lit_t = PRESS_FRAMES;
        if (S.seq[S.pos] != b)
            return false;
        if (++S.pos == S.len) {
            S.score = S.len;
            S.state = GOOD;
            S.t = 0;
        }
        return true;
    }
    default: /* GOOD */
        if (S.t >= GOOD_FRAMES)
            add_step();
        return true;
    }
}

void simon_draw(uint32_t t)
{
    (void)t;
    draw_number(S.len, 5, S.state == SHOW ? 80 : 255);
    for (uint8_t b = 0; b < 4; b++) {
        uint8_t arm = BUTTON_ARM[b];
        bool on = S.lit == b;
        arm_sides(arm, rgb_scaled(COLOUR[arm], on ? 255 : 50));
        if (on)
            for (int k = 0; k < ARM_LINE; k++)
                arm_line(arm, k, COLOUR[arm]);
    }
    if (S.state == GOOD) {
        for (int k = 0; k < ARM_LINE; k++)
            arm_line(0, k, (rgb_t){0, 255, 60});
    } else if (S.state == INPUT) { /* how far into the sequence */
        for (int k = 0; k < ARM_LINE; k++)
            arm_line(0, k, k < S.pos * ARM_LINE / S.len ? (rgb_t){200, 200, 200} : (rgb_t){20, 20, 20});
    }
}

uint16_t simon_score(void)
{
    return S.score;
}
