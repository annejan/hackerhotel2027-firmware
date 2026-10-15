/* Shared game shell (same as ../../src/game.rs). Every game starts in demo
 * mode (it plays itself); pressing ◀, B or ▶ starts a real game, and
 * game over shows the score. A stays free in demo mode, so holding it
 * still opens the menu.
 *
 * A game provides init / tick (one frame, false at game over) / draw /
 * score, see games.h. Input arrives as bitmasks of the buttons: pressed since
 * the last frame and held right now (bit n = button n: ◀ B A ▶). */
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "events.h"
#include "frame.h"

#define BTN_LEFT (1u << 0)
#define BTN_B (1u << 1)
#define BTN_A (1u << 2)
#define BTN_RIGHT (1u << 3)

typedef enum {
    GAME_SNAKE, GAME_STARTRIS, GAME_BREAKOUT, GAME_PONG, GAME_FLAPPY, GAME_CHASE,
    GAME_WHACK, GAME_SIMON, GAME_MAZE, GAME_HILO, /* the challenges' own games (challenge.h) */
    GAME_FOUR, /* four in a row, also against another badge over infrared */
    GAMES
} game_kind_t;

typedef struct {
    uint8_t pressed, held;
} input_t;

/* new game of this kind, in demo mode */
void game_start(game_kind_t kind);
/* returns true when the event was used by the game */
bool game_event(event_t ev, uint8_t arg);
/* a real game is going on (not the demo) */
bool game_playing(void);
/* one frame into fb[]; held = buttons down right now */
void game_render(uint8_t held);
/* a game packet from another badge (IR_FOUR) */
void game_ir(const uint8_t *data, int len);

/* helpers for the games */
/* light arm `arm`'s first n LEDs (centre outward) in c */
void arm_fill(uint8_t arm, int n, rgb_t c);
/* a score of up to 35 as 7 LEDs per arm, filling arm 0 first */
void arms_score(uint16_t score, rgb_t c);
/* The five LEDs of an arm that stand in a line, centre outward: ranks 0,
 * 3, 4, 5, 6 (ranks 1 and 2 sit beside rank 0, at the arm's root). */
#define ARM_LINE 5
extern const uint8_t ARM_LINE_RANK[ARM_LINE];
/* light LED k (0 = centre) of arm `arm`'s line in c */
void arm_line(uint8_t arm, int k, rgb_t c);
/* light the two LEDs beside the line, at the arm's root */
void arm_sides(uint8_t arm, rgb_t c);
/* the arm a button sits under: ◀ 3, B 4, A 1, ▶ 2 (arm 0, the top, has none) */
extern const uint8_t BUTTON_ARM[4];
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
