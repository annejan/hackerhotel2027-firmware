/* The challenges (C firmware; same as ../../src/challenge.rs): five
 * games, one per arm, five levels each, a level for every LED in the arm's
 * line. All 25 and the badge does its victory dance.
 *
 *   arm 0 (top)          MAZE          mazes solved: 1 2 3 4 5
 *   arm 1 (A's arm)      SIMON         sequence repeated: 4 6 8 10 12
 *   arm 2 (▶'s leg)      WHACK-A-MOLE  points: 20 40 60 80 100
 *   arm 3 (◀'s leg)      HIGHER LOWER  streak: 3 5 7 9 12
 *   arm 4 (B's arm)      SNAKE         food eaten: 5 10 15 20 30
 *
 * A level counts when a real game (not the demo) ends with that score.
 * The name tag shows the progress on the arms, so people can compare.
 * main.c keeps it in flash (store.h); it only ever goes up. */
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "game.h"

#define CHALLENGES 5
#define CHALLENGE_LEVELS 5

/* progress as kept in flash: 3 bits a challenge */
void challenge_load(uint16_t packed);
uint16_t challenge_packed(void);
/* the progress changed since the last call (for main.c to save it) */
bool challenge_take_changed(void);

/* a real game of `kind` ended with `score`: the challenge (= arm) that
 * went up a level, or -1 */
int challenge_result(game_kind_t kind, uint16_t score);
uint8_t challenge_level(int c);
const char *challenge_name(int c);
int challenge_total(void); /* 0..25 */
/* all 25 were just reached: the victory dance is due (once) */
bool challenge_victory_due(void);
void challenge_victory_done(void);

/* the progress on the arms; `fresh` (an arm, or -1) blinks its newest level */
void challenge_arms(uint32_t t, int fresh);

/* the victory dance: a stick figure dancing on the grid, with disco
 * colours on the arms, and between dances the text a phrase at a time,
 * with fireworks. Returns false when it is over. */
void victory_start(void);
bool victory_render(void);
