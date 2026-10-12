/* The games' state, and their entry points for the shell in game.c. Only
 * one game runs at a time, so their states share one union. */
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "game.h"

#define SNAKE_MAX 208 /* every LED in the grid */
#define WELL_H 14
#define BRICK_ROWS 5
#define PIPES 4

typedef struct {
    uint16_t body[SNAKE_MAX]; /* ring buffer of cells (row * GRID_W + col) */
    uint16_t head, len;
    uint32_t occ[GRID_H]; /* occupied cells, one bit per column */
    uint8_t dir;
    int8_t turns[2]; /* pressed but not yet taken: +1 right, -1 left */
    uint8_t nturns;
    uint16_t food, score;
    uint32_t tick;
    uint8_t flash;
} snake_t;

typedef struct {
    uint16_t well[WELL_H]; /* bit c = column c filled */
    uint8_t kind, rot, next, bag;
    int r, c;
    uint32_t fall;
    uint16_t lines, clearing; /* rows being cleared, bit per row */
    uint8_t clear_t;
    uint8_t rep[2]; /* frames T1 / T4 held, for auto-repeat */
    uint8_t target_rot;
    int target_c;
} startris_t;

typedef struct {
    uint32_t bricks[BRICK_ROWS];
    int paddle, y, x, vy, vx, speed;
    uint8_t serve, lives, flash;
    uint16_t score;
    uint32_t t;
} breakout_t;

typedef struct {
    int left, right, y, x, vy, vx;
    uint8_t points[2], serve;
    int aim_err[2];
    uint8_t flash, flash_who;
    uint32_t t;
} pong_t;

typedef struct {
    int y, vy;
    struct {
        int x; /* column, fixed point; < 0 = unused */
        int gap;
        bool passed;
    } pipes[PIPES];
    uint16_t score;
    uint32_t t;
} flappy_t;

typedef struct {
    int pos, speed; /* outline position and speed, 1/256 LED */
    uint8_t arm, lives, fx;
    bool fx_hit;
    uint16_t hits;
    uint32_t t;
} chase_t;

extern union game_state {
    snake_t snake;
    startris_t startris;
    breakout_t breakout;
    pong_t pong;
    flappy_t flappy;
    chase_t chase;
} game_state;

#define GAME_FNS(name)                                  \
    void name##_init(void);                             \
    bool name##_tick(bool demo, input_t in);            \
    void name##_draw(uint32_t t);                       \
    uint16_t name##_score(void);

GAME_FNS(snake)
GAME_FNS(startris)
GAME_FNS(breakout)
GAME_FNS(pong)
GAME_FNS(flappy)
GAME_FNS(chase)
bool pong_won(void);
