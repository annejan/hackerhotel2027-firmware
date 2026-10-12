/* Shared game shell (same as ../../src/game.rs). Every game starts in demo
 * mode (it plays itself); touching T1, T2 or T4 starts a real game, and
 * game over shows the score. T3 stays free in demo mode, so holding it
 * still opens the menu.
 *
 * A game provides init / tick (one frame, false at game over) / draw /
 * score, see games.h. Input arrives as bitmasks of the pads: pressed since
 * the last frame and held right now (bit n = pad Tn+1). */
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "events.h"
#include "frame.h"

#define PAD_T1 (1u << 0)
#define PAD_T2 (1u << 1)
#define PAD_T3 (1u << 2)
#define PAD_T4 (1u << 3)

typedef enum { GAME_SNAKE, GAME_STARTRIS, GAME_BREAKOUT, GAME_PONG, GAME_FLAPPY, GAME_CHASE, GAMES } game_kind_t;

typedef struct {
    uint8_t pressed, held;
} input_t;

/* new game of this kind, in demo mode */
void game_start(game_kind_t kind);
/* returns true when the event was used by the game */
bool game_event(event_t ev, uint8_t arg);
/* a real game is going on (not the demo) */
bool game_playing(void);
/* one frame into fb[]; held = pads touched right now */
void game_render(uint8_t held);

/* helpers for the games */
/* light arm `arm`'s first n LEDs (centre outward) in c */
void arm_fill(uint8_t arm, int n, rgb_t c);
/* a score of up to 35 as 7 LEDs per arm, filling arm 0 first */
void arms_score(uint16_t score, rgb_t c);
/* step *pos towards target by one, at most once every `every` frames */
int approach(int *pos, int target, uint32_t t, uint32_t every);

static inline int clampi(int v, int lo, int hi)
{
    return v < lo ? lo : v > hi ? hi : v;
}

static inline int absi(int v)
{
    return v < 0 ? -v : v;
}

/* a mod n, always 0..n-1 */
static inline int wrapi(int a, int n)
{
    a %= n;
    return a < 0 ? a + n : a;
}
