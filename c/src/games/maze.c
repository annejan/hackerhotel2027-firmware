/* Maze, same as ../../../src/games/maze.rs: you stay in the middle of the
 * grid and the maze moves around you; only the walls close by are lit, so
 * you see one or two corridors ahead. ◀ / ▶ go left / right, B up, A down
 * (held, they repeat). The arm pointing towards the exit lights up, and
 * the outline shows the time left. Five mazes, from 5 x 5 to 13 x 13
 * cells; each one solved is a level of challenge 0 (challenge.c). */
#include <string.h>
#include "anim.h"
#include "games.h"

#define S (game_state.maze)
#define MAZES 5
#define ME_R 7 /* where the player stands on the grid */
#define ME_C 8
#define REPEAT_AFTER 8
#define REPEAT_EVERY 4
#define SOLVED_FX FPS

static const int8_t DR[4] = {0, 1, 0, -1}; /* right, down, left, up */
static const int8_t DC[4] = {1, 0, -1, 0};
static const uint8_t KEY_DIR[4] = {2, 3, 1, 0}; /* ◀ left, B up, A down, ▶ right */

static bool wall(int r, int c)
{
    if (r < 0 || c < 0 || r >= S.size || c >= S.size)
        return false;
    return S.wall[r] >> c & 1;
}

static void open_px(int r, int c)
{
    S.wall[r] &= ~(1u << c);
}

/* a perfect maze (one way between any two cells): depth-first, carving */
static void make(void)
{
    int n = 5 + 2 * S.solved;
    S.size = (uint8_t)(2 * n + 1);
    for (int r = 0; r < MAZE_PX; r++)
        S.wall[r] = r < S.size ? (1u << S.size) - 1 : 0;
    int top = 0;
    S.stack[top++] = 0;
    open_px(1, 1);
    while (top) {
        int cell = S.stack[top - 1], cr = cell / n, cc = cell % n;
        int options[4], k = 0;
        for (int d = 0; d < 4; d++) {
            int nr = cr + DR[d], nc = cc + DC[d];
            if (nr >= 0 && nc >= 0 && nr < n && nc < n && wall(2 * nr + 1, 2 * nc + 1))
                options[k++] = d;
        }
        if (!k) {
            top--;
            continue;
        }
        int d = options[rng_next() % (uint32_t)k];
        open_px(2 * cr + 1 + DR[d], 2 * cc + 1 + DC[d]);
        open_px(2 * (cr + DR[d]) + 1, 2 * (cc + DC[d]) + 1);
        S.stack[top++] = (uint8_t)((cr + DR[d]) * n + cc + DC[d]);
    }
    S.r = S.c = 1;
    S.dir = 0;
    S.limit = S.time = (uint16_t)((15 + 8 * S.solved) * FPS);
}

void maze_init(void)
{
    memset(&S, 0, sizeof S);
    make();
}

bool maze_won(void)
{
    return S.solved >= MAZES;
}

static bool step(int d)
{
    if (wall(S.r + DR[d], S.c + DC[d]))
        return false;
    S.r = (uint8_t)(S.r + 2 * DR[d]);
    S.c = (uint8_t)(S.c + 2 * DC[d]);
    return true;
}

bool maze_tick(bool demo, input_t in)
{
    if (S.fx) {
        if (--S.fx == 0)
            make();
        return true;
    }
    if (S.time && --S.time == 0)
        return false;
    if (demo) { /* keep a hand on the right-hand wall */
        if (S.time % 5 == 0)
            for (int k = 0; k < 4; k++) {
                int d = (S.dir + 1 + 3 * k) % 4; /* right, straight, left, back */
                if (step(d)) {
                    S.dir = (uint8_t)d;
                    break;
                }
            }
    } else {
        int key = -1;
        for (int k = 0; k < 4 && key < 0; k++)
            if (in.pressed >> k & 1)
                key = k;
        if (key >= 0) {
            S.rep = 0;
        } else {
            for (int k = 0; k < 4 && key < 0; k++)
                if (in.held >> k & 1)
                    key = k;
            if (key >= 0 && !(++S.rep >= REPEAT_AFTER && S.rep % REPEAT_EVERY == 0))
                key = -1;
        }
        if (key >= 0)
            step(KEY_DIR[key]);
    }
    if (S.r == S.size - 2 && S.c == S.size - 2) {
        if (++S.solved >= MAZES)
            return false;
        S.fx = SOLVED_FX;
    }
    return true;
}

void maze_draw(uint32_t t)
{
    if (S.fx) {
        draw_number(S.solved, 5, 255);
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, (rgb_t){0, 200, 60});
        return;
    }
    int er = S.size - 2, ec = S.size - 2; /* the exit: the far corner */
    for (int r = 0; r < GRID_H; r++)
        for (int c = 0; c < GRID_W; c++) {
            int mr = S.r + r - ME_R, mc = S.c + c - ME_C;
            int d = absi(r - ME_R) > absi(c - ME_C) ? absi(r - ME_R) : absi(c - ME_C);
            if (wall(mr, mc) && d <= 4)
                fb_grid(r, c, d <= 2 ? 255 : d == 3 ? 110 : 35);
            else if (mr == er && mc == ec && d <= 5)
                fb_grid(r, c, t % 10 < 5 ? 255 : 60);
        }
    fb_grid(ME_R, ME_C, t % 16 < 12 ? 255 : 90);
    /* compass: the arm that points most towards the exit */
    int dx = ec - S.c, dy = er - S.r;
    int best = 0, best_dot = -(1 << 30);
    for (int a = 0; a < 5; a++) {
        uint8_t ang = (uint8_t)(a * 256 / 5);
        int dot = dx * (sin8(ang) - 128) - dy * (sin8((uint8_t)(ang + 64)) - 128);
        if (dot > best_dot) {
            best_dot = dot;
            best = a;
        }
    }
    for (uint8_t a = 0; a < 5; a++)
        arm_sides(a, a < S.solved ? (rgb_t){0, 160, 40} : (rgb_t){10, 10, 30});
    for (int k = 0; k < ARM_LINE; k++)
        arm_line((uint8_t)best, k, rgb_scaled((rgb_t){60, 120, 255}, (uint8_t)(80 + 35 * k)));
    /* time left, round the outline */
    int left = (int)((uint32_t)S.time * EDGE_LEDS / S.limit);
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = j < left ? (S.time < 5 * FPS && t % 10 < 5 ? 20 : 70) : 0;
}

uint16_t maze_score(void)
{
    return S.solved;
}
