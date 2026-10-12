/* Pong against the badge, same as ../../../src/games/pong.rs: your paddle
 * on the left edge (T1 up, T4 down, hold to keep moving), the badge's on
 * the right. First to 5 wins. The left arms show your points in blue, the
 * right arms the badge's in red. */
#include <string.h>
#include "games.h"

#define S (game_state.pong)
#define ONE 256
#define RIGHT (GRID_W - 1)
#define TOP 5 /* paddles move over rows 5..10, 2 tall */
#define BOTTOM 9
#define WIN 5

static const rgb_t BLUE = {20, 90, 255};
static const rgb_t RED = {255, 20, 20};

static inline int cell(int v) { return (v + ONE / 2) >> 8; }

static void serve(int towards)
{
    S.y = 7 * ONE + ONE / 2;
    S.x = 8 * ONE;
    S.vx = towards * ONE * 2 / 7;
    S.vy = (rng_next() & 1) == 0 ? ONE / 8 : -ONE / 8;
    S.serve = 20;
    S.aim_err[0] = S.aim_err[1] = 0;
}

/* where a computer paddle wants its top row */
static int aim(int side)
{
    bool coming = side == 0 ? S.vx < 0 : S.vx > 0;
    int y = coming ? S.y + S.aim_err[side] : 7 * ONE;
    return clampi(cell(y) - 1, TOP, BOTTOM);
}

void pong_init(void)
{
    memset(&S, 0, sizeof S);
    S.left = S.right = 7;
    serve(1);
}

bool pong_tick(bool demo, input_t in)
{
    uint32_t t = ++S.t;
    if (S.flash)
        S.flash--;

    if (demo)
        approach(&S.left, aim(0), t, 3);
    else if (in.pressed & PAD_T1 || (in.held & PAD_T1 && t % 3 == 0))
        S.left = S.left - 1 < TOP ? TOP : S.left - 1;
    else if (in.pressed & PAD_T4 || (in.held & PAD_T4 && t % 3 == 0))
        S.left = S.left + 1 > BOTTOM ? BOTTOM : S.left + 1;
    approach(&S.right, aim(1), t + 1, 3);

    if (S.serve) {
        S.serve--;
        return true;
    }
    int ny = S.y + S.vy, nx = S.x + S.vx;
    if (!on_grid(cell(ny), clampi(cell(S.x), 0, RIGHT)))
        S.vy = -S.vy;
    for (int side = 0; side < 2; side++) {
        int top = side == 0 ? S.left : S.right;
        int line = (side == 0 ? 1 : RIGHT - 1) * ONE;
        bool moving = side == 0 ? S.vx < 0 : S.vx > 0;
        bool crosses = (S.x - line) * (nx - line) <= 0;
        bool covered = S.y >= top * ONE - ONE * 3 / 5 && S.y <= (top + 1) * ONE + ONE * 3 / 5;
        if (moving && crosses && covered) {
            S.vx = -S.vx * 26 / 25;
            S.vy = clampi(S.vy + (S.y - top * ONE - ONE / 2) / 10, -ONE / 3, ONE / 3);
            /* the other side's computer player may misjudge this one */
            S.aim_err[1 - side] = rng_next() % 4 == 0 ? ONE * 5 / 2 : 0;
        }
    }
    S.y += S.vy;
    S.x += S.vx;

    if (S.x < -ONE / 2 || S.x > RIGHT * ONE + ONE / 2) {
        int who = S.x < 0 ? 1 : 0;
        S.points[who]++;
        S.flash = 16;
        S.flash_who = (uint8_t)who;
        if (S.points[who] == WIN)
            return false;
        serve(who == 0 ? 1 : -1);
    }
    return true;
}

void pong_draw(uint32_t t)
{
    for (int r = 0; r < 16; r += 2)
        fb_grid(r, 8, 18); /* the net */
    for (int d = 0; d < 2; d++) {
        fb_grid(S.left + d, 0, 255);
        fb_grid(S.right + d, RIGHT, 255);
    }
    if (!S.serve || t % 6 < 3)
        fb_grid(clampi(cell(S.y), 0, 15), clampi(cell(S.x), 0, RIGHT), 255);
    /* left arms (3, 4) are yours, right arms (1, 2) the badge's */
    static const uint8_t ARMS[2][2] = {{4, 3}, {1, 2}};
    for (int who = 0; who < 2; who++) {
        rgb_t c = who == 0 ? BLUE : RED;
        bool flashing = S.flash && S.flash_who == who;
        for (int k = 0; k < 2; k++) {
            if (flashing)
                arm_fill(ARMS[who][k], 7, rgb_scaled(c, (uint8_t)(S.flash * 15)));
            else
                arm_fill(ARMS[who][k], S.points[who], rgb_scaled(c, 220));
        }
    }
    arm_fill(0, 7, rgb_scaled((rgb_t){255, 255, 255}, 25));
}

uint16_t pong_score(void)
{
    return S.points[0];
}

bool pong_won(void)
{
    return S.points[0] == WIN;
}
