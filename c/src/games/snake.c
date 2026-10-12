/* Snake on the 17 x 16 grid, same as ../../../src/games/snake.rs. The
 * octagon's edge is the wall. T1 turns left, T4 turns right. The demo
 * steers by the shortest path to the food. The arms fill green. */
#include <string.h>
#include "games.h"

#define S (game_state.snake)
#define CELLS (GRID_H * GRID_W)

/* right, down, left, up; turning left is dir + 3, right is dir + 1 */
static const int8_t DR[4] = {0, 1, 0, -1};
static const int8_t DC[4] = {1, 0, -1, 0};

static inline uint16_t cell(int r, int c) { return (uint16_t)(r * GRID_W + c); }
static inline int row(uint16_t cl) { return cl / GRID_W; }
static inline int col(uint16_t cl) { return cl % GRID_W; }

static bool occupied(uint16_t cl) { return S.occ[row(cl)] >> col(cl) & 1; }

static void mark(uint16_t cl, bool on)
{
    if (on)
        S.occ[row(cl)] |= 1u << col(cl);
    else
        S.occ[row(cl)] &= ~(1u << col(cl));
}

static void push(uint16_t cl)
{
    S.head = (S.head + 1) % SNAKE_MAX;
    S.body[S.head] = cl;
    S.len++;
    mark(cl, true);
}

static uint16_t tail(void)
{
    return S.body[(S.head + SNAKE_MAX + 1 - S.len) % SNAKE_MAX];
}

static uint16_t new_food(void)
{
    uint32_t k = rng_next() % (SNAKE_MAX - S.len ? SNAKE_MAX - S.len : 1);
    for (int r = 0; r < GRID_H; r++)
        for (int c = 0; c < GRID_W; c++)
            if (on_grid(r, c) && !occupied(cell(r, c)) && k-- == 0)
                return cell(r, c);
    return tail(); /* board full; never reached in practice */
}

void snake_init(void)
{
    memset(&S, 0, sizeof S);
    S.head = SNAKE_MAX - 1;
    for (int c = 4; c <= 6; c++)
        push(cell(8, c));
    S.food = new_food();
}

/* demo: first step of a shortest path to the food, avoiding the body (the
 * tail cell is fine, it moves away) */
static uint8_t autopilot(void)
{
    static uint8_t from[CELLS]; /* direction we came in with, 0xFF = unseen */
    static uint16_t queue[SNAKE_MAX];
    uint16_t head = S.body[S.head], tl = tail();
    int qh = 0, qt = 0;
    memset(from, 0xFF, sizeof from);
    queue[qt++] = head;
    from[head] = 4;
    while (qh < qt) {
        uint16_t p = queue[qh++];
        if (p == S.food) {
            for (;;) { /* walk back to the cell next to the head */
                uint8_t d = from[p];
                uint16_t back = cell(row(p) - DR[d], col(p) - DC[d]);
                if (back == head)
                    return d;
                p = back;
            }
        }
        for (uint8_t d = 0; d < 4; d++) {
            int nr = row(p) + DR[d], nc = col(p) + DC[d];
            if (!on_grid(nr, nc))
                continue;
            uint16_t n = cell(nr, nc);
            if (from[n] == 0xFF && (!occupied(n) || n == tl) && qt < SNAKE_MAX) {
                from[n] = d;
                queue[qt++] = n;
            }
        }
    }
    /* no path: straight on if safe, else whichever turn is */
    const uint8_t tries[3] = {S.dir, (uint8_t)((S.dir + 3) % 4), (uint8_t)((S.dir + 1) % 4)};
    for (int i = 0; i < 3; i++) {
        int nr = row(head) + DR[tries[i]], nc = col(head) + DC[tries[i]];
        if (on_grid(nr, nc) && !occupied(cell(nr, nc)))
            return tries[i];
    }
    return S.dir;
}

/* one move; false on a crash */
static bool step(bool demo)
{
    if (demo) {
        S.dir = autopilot();
    } else if (S.nturns) {
        S.dir = (uint8_t)((S.dir + 4 + S.turns[0]) % 4);
        S.turns[0] = S.turns[1];
        S.nturns--;
    }
    uint16_t head = S.body[S.head];
    int nr = row(head) + DR[S.dir], nc = col(head) + DC[S.dir];
    if (!on_grid(nr, nc))
        return false;
    uint16_t next = cell(nr, nc);
    bool eating = next == S.food;
    if (occupied(next) && (eating || next != tail()))
        return false;
    if (!eating) {
        mark(tail(), false);
        S.len--;
    }
    push(next);
    if (eating) {
        S.score++;
        S.flash = 8;
        S.food = new_food();
    }
    return true;
}

bool snake_tick(bool demo, input_t in)
{
    if (in.pressed & PAD_T1 && S.nturns < 2)
        S.turns[S.nturns++] = -1;
    if (in.pressed & PAD_T4 && S.nturns < 2)
        S.turns[S.nturns++] = 1;
    S.tick++;
    if (S.flash)
        S.flash--;
    uint32_t every = S.score >= 10 ? 3 : 4; /* 7.5 moves/s, 10 from a score of 10 */
    return S.tick % every != 0 || step(demo);
}

void snake_draw(uint32_t t)
{
    for (int k = 0; k < S.len; k++) {
        uint16_t cl = S.body[(S.head + SNAKE_MAX - k) % SNAKE_MAX];
        int v = k == 0 ? 255 : 200 - k * 5 < 90 ? 90 : 200 - k * 5;
        fb_grid(row(cl), col(cl), (uint8_t)v);
    }
    if (t % 6 < 4)
        fb_grid(row(S.food), col(S.food), 230);
    arms_score(S.score, rgb_scaled((rgb_t){25, 255, 60}, 180));
    if (S.flash) {
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, rgb_scaled((rgb_t){50, 255, 80}, (uint8_t)(S.flash * 31)));
        for (int j = 0; j < EDGE_LEDS; j++)
            fb[EDGE[j]] = (uint8_t)(S.flash * 25);
    }
}

uint16_t snake_score(void)
{
    return S.score;
}
