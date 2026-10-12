/* Breakout, same as ../../../src/games/breakout.rs: five rows of bricks on
 * top, a 3-wide paddle on the bottom row, the octagon's angled walls to
 * bounce off. T1 / T4 move the paddle (hold to keep moving). 3 lives on
 * arms 0..2; the score fills orange. */
#include <string.h>
#include "games.h"

#define S (game_state.breakout)
#define ONE 256 /* fixed point: 1/256 cell */
#define PADDLE_ROW 14 /* grid columns 4..12 on this row */
#define SERVE_FRAMES 20

static inline int cell(int v) { return (v + ONE / 2) >> 8; }

static void fill(void)
{
    for (int i = 0; i < BRICK_ROWS; i++)
        S.bricks[i] = MASK[i + 1];
}

static bool brick(int r, int c)
{
    return r >= 1 && r <= BRICK_ROWS && c >= 0 && S.bricks[r - 1] >> c & 1;
}

static void serve(void)
{
    S.serve = SERVE_FRAMES;
    S.vy = -S.speed;
    S.vx = S.speed / 3;
}

void breakout_init(void)
{
    memset(&S, 0, sizeof S);
    S.paddle = 8;
    S.lives = 3;
    S.speed = ONE / 2; /* 15 cells per second */
    fill();
    serve();
}

bool breakout_tick(bool demo, input_t in)
{
    S.t++;
    if (S.flash)
        S.flash--;

    /* paddle: the demo follows the ball, the player holds T1 / T4 */
    int target = demo                                                                  ? cell(S.x)
                 : in.pressed & PAD_T1 || (in.held & PAD_T1 && S.t % 3 == 0) ? S.paddle - 1
                 : in.pressed & PAD_T4 || (in.held & PAD_T4 && S.t % 3 == 0) ? S.paddle + 1
                                                                                     : S.paddle;
    approach(&S.paddle, clampi(target, 5, 11), S.t, demo ? 2 : 1);

    if (S.serve) {
        S.serve--;
        S.y = (PADDLE_ROW - 1) * ONE;
        S.x = S.paddle * ONE;
        return true;
    }

    int ny = S.y + S.vy, nx = S.x + S.vx;
    int r = cell(ny), c = cell(nx);
    if (brick(r, c)) {
        S.bricks[r - 1] &= ~(1u << c);
        S.score++;
        S.flash = 6;
        S.vy = -S.vy;
    } else if (r < PADDLE_ROW) {
        if (!on_grid(r, cell(S.x)))
            S.vy = -S.vy;
        if (!on_grid(cell(S.y), c))
            S.vx = -S.vx;
    }
    /* paddle: bounce, and steer by where the ball hits it */
    int edge = PADDLE_ROW * ONE - ONE / 2;
    if (S.vy > 0 && S.y < edge && S.y + S.vy >= edge && absi(S.x - S.paddle * ONE) <= ONE * 8 / 5) {
        S.vy = -absi(S.vy);
        S.vx = clampi(S.vx + (S.x - S.paddle * ONE) / 10, -S.speed * 3 / 5, S.speed * 3 / 5);
    }
    S.y += S.vy;
    S.x += S.vx;

    if (S.y > (PADDLE_ROW + 1) * ONE + ONE / 2) {
        if (--S.lives == 0)
            return false;
        serve();
    }
    bool cleared = true;
    for (int i = 0; i < BRICK_ROWS; i++)
        cleared &= S.bricks[i] == 0;
    if (cleared) {
        fill(); /* next wave, a bit faster */
        S.speed += ONE / 16;
        serve();
    }
    return true;
}

void breakout_draw(uint32_t t)
{
    for (int r = 0; r < BRICK_ROWS; r++)
        for (int c = 0; c < GRID_W; c++)
            if (S.bricks[r] >> c & 1)
                fb_grid(r + 1, c, 130);
    for (int d = -1; d <= 1; d++)
        fb_grid(PADDLE_ROW, S.paddle + d, 210);
    if (!S.serve || t % 6 < 3)
        fb_grid(cell(S.y), cell(S.x), 255);
    for (uint8_t a = 0; a < S.lives; a++)
        arm_fill(a, 7, rgb_scaled((rgb_t){20, 70, 255}, 70));
    arms_score(S.score / 3, rgb_scaled((rgb_t){255, 150, 20}, 220));
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = (uint8_t)(S.flash * 30);
}

uint16_t breakout_score(void)
{
    return S.score;
}
