/* Flappy, same as ../../../src/games/flappy.rs: one button. Press ◀, B
 * or ▶ to flap through the gaps. The arms fill gold with the pipes you
 * pass. */
#include <string.h>
#include "games.h"

#define S (game_state.flappy)
#define ONE 256
#define BIRD_COL 4
#define GAP 4 /* rows */
#define GRAVITY 6
#define FLAP (-72)
#define MAX_FALL 100
#define SPEED 57 /* columns per frame, 1/256: about 6.7 per second */
#define EVERY 36 /* frames between pipes */

static inline int cell(int v) { return (v + ONE / 2) >> 8; }

static bool in_gap(int gap, int r) { return r >= gap && r < gap + GAP; }

static int next_gap(void)
{
    int best = -1;
    for (int i = 0; i < PIPES; i++)
        if (S.pipes[i].x >= 0 && S.pipes[i].x + ONE / 2 >= BIRD_COL * ONE &&
            (best < 0 || S.pipes[i].x < S.pipes[best].x))
            best = i;
    return best < 0 ? 7 : S.pipes[best].gap;
}

void flappy_init(void)
{
    memset(&S, 0, sizeof S);
    S.y = 7 * ONE;
    for (int i = 0; i < PIPES; i++)
        S.pipes[i].x = -1;
}

bool flappy_tick(bool demo, input_t in)
{
    S.t++;
    if (S.t % EVERY == 1)
        for (int i = 0; i < PIPES; i++)
            if (S.pipes[i].x < 0) {
                S.pipes[i].x = (GRID_W + 1) * ONE;
                S.pipes[i].gap = 3 + (int)(rng_next() % 6);
                S.pipes[i].passed = false;
                break;
            }
    for (int i = 0; i < PIPES; i++) {
        if (S.pipes[i].x < 0)
            continue;
        S.pipes[i].x -= SPEED;
        if (S.pipes[i].x < 0)
            S.pipes[i].x = -1;
        if (!S.pipes[i].passed && S.pipes[i].x < BIRD_COL * ONE - ONE / 2) {
            S.pipes[i].passed = true;
            S.score++;
        }
    }

    bool flap;
    if (demo) {
        /* flap when about to sink below the middle of the next gap */
        int mid = next_gap() * ONE + GAP * ONE / 2 - ONE / 2;
        flap = S.vy >= 0 && S.y + S.vy * 3 > mid + ONE * 2 / 5;
    } else {
        flap = in.pressed & (BTN_LEFT | BTN_B | BTN_RIGHT);
    }
    if (flap)
        S.vy = FLAP;
    S.vy = S.vy + GRAVITY > MAX_FALL ? MAX_FALL : S.vy + GRAVITY;
    S.y += S.vy;

    int r = cell(S.y);
    for (int i = 0; i < PIPES; i++)
        if (S.pipes[i].x >= 0 && absi(S.pipes[i].x - BIRD_COL * ONE) < ONE * 3 / 5 && !in_gap(S.pipes[i].gap, r))
            return false;
    return on_grid(r, BIRD_COL);
}

void flappy_draw(uint32_t t)
{
    (void)t;
    for (int i = 0; i < PIPES; i++) {
        if (S.pipes[i].x < 0)
            continue;
        int c = cell(S.pipes[i].x);
        for (int r = 0; r < GRID_H; r++)
            if (!in_gap(S.pipes[i].gap, r) && c >= 0 && c < GRID_W)
                fb_grid(r, c, 100);
    }
    int r = cell(S.y);
    if (r >= 0 && r < GRID_H) {
        fb_grid(r, BIRD_COL, 255);
        fb_grid(clampi(r + (S.vy < 0 ? 1 : -1), 0, GRID_H - 1), BIRD_COL - 1, 80);
    }
    arms_score(S.score, rgb_scaled((rgb_t){255, 190, 20}, 220));
}

uint16_t flappy_score(void)
{
    return S.score;
}
