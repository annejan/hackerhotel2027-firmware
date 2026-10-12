/* Snake, same as the Rust firmware's snake.rs. */
#include <string.h>
#include "anim.h"
#include "font.h"
#include "frame.h"
#include "snake.h"

#define CELLS (GRID_H * GRID_W)
#define MAX_LEN 208 /* every LED in the grid */
#define CRASH_FRAMES 20
#define SCORE_FRAMES (2 * FPS)

enum { ATTRACT, PLAY, CRASH, SCORE };

/* right, down, left, up; turning left is dir + 3, right is dir + 1 */
static const int8_t DR[4] = {0, 1, 0, -1};
static const int8_t DC[4] = {1, 0, -1, 0};

static struct {
    uint8_t state, timer; /* timer counts down in CRASH and SCORE */
    bool demo; /* attract mode (including its score pause), not a game */
    uint16_t body[MAX_LEN]; /* ring buffer of cells (row * GRID_W + col) */
    uint16_t head, len;
    uint32_t occ[GRID_H]; /* occupied cells, one bit per column */
    uint8_t dir;
    int8_t turns[2]; /* pressed but not yet taken: +1 right, -1 left */
    uint8_t nturns;
    uint16_t food, score;
    uint32_t tick;
    uint8_t flash;
} s;

static bool on_grid(int r, int c)
{
    return r >= 0 && r < GRID_H && c >= 0 && c < GRID_W && GRID[r][c] != SLOT_NONE;
}

static inline uint16_t cell(int r, int c) { return (uint16_t)(r * GRID_W + c); }
static inline int row(uint16_t cl) { return cl / GRID_W; }
static inline int col(uint16_t cl) { return cl % GRID_W; }

static bool occupied(uint16_t cl) { return s.occ[row(cl)] >> col(cl) & 1; }

static void mark(uint16_t cl, bool on)
{
    if (on)
        s.occ[row(cl)] |= 1u << col(cl);
    else
        s.occ[row(cl)] &= ~(1u << col(cl));
}

static void push(uint16_t cl)
{
    s.head = (s.head + 1) % MAX_LEN;
    s.body[s.head] = cl;
    s.len++;
    mark(cl, true);
}

static uint16_t tail(void)
{
    return s.body[(s.head + MAX_LEN + 1 - s.len) % MAX_LEN];
}

static uint16_t new_food(void)
{
    uint32_t k = rng_next() % (MAX_LEN - s.len ? MAX_LEN - s.len : 1);
    for (int r = 0; r < GRID_H; r++)
        for (int c = 0; c < GRID_W; c++)
            if (on_grid(r, c) && !occupied(cell(r, c)) && k-- == 0)
                return cell(r, c);
    return tail(); /* board full; never reached in practice */
}

static void start(uint8_t state)
{
    memset(&s, 0, sizeof s);
    s.state = state;
    s.demo = state == ATTRACT;
    s.head = MAX_LEN - 1;
    for (int c = 4; c <= 6; c++)
        push(cell(8, c));
    s.food = new_food();
}

void snake_reset(void)
{
    start(ATTRACT);
}

/* attract mode: first step of a shortest path to the food, avoiding the
 * body (the tail cell is fine, it moves away) */
static uint8_t autopilot(void)
{
    static uint8_t from[CELLS]; /* direction we came in with, 0xFF = unseen */
    static uint16_t queue[MAX_LEN];
    uint16_t head = s.body[s.head], tl = tail();
    int qh = 0, qt = 0;
    memset(from, 0xFF, sizeof from);
    queue[qt++] = head;
    from[head] = 4;
    while (qh < qt) {
        uint16_t p = queue[qh++];
        if (p == s.food) {
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
            if (from[n] == 0xFF && (!occupied(n) || n == tl) && qt < MAX_LEN) {
                from[n] = d;
                queue[qt++] = n;
            }
        }
    }
    /* no path: straight on if safe, else whichever turn is */
    const uint8_t tries[3] = {s.dir, (uint8_t)((s.dir + 3) % 4), (uint8_t)((s.dir + 1) % 4)};
    for (int i = 0; i < 3; i++) {
        int nr = row(head) + DR[tries[i]], nc = col(head) + DC[tries[i]];
        if (on_grid(nr, nc) && !occupied(cell(nr, nc)))
            return tries[i];
    }
    return s.dir;
}

/* one move; false on a crash */
static bool step(void)
{
    if (s.state == ATTRACT) {
        s.dir = autopilot();
    } else if (s.nturns) {
        s.dir = (uint8_t)((s.dir + 4 + s.turns[0]) % 4);
        s.turns[0] = s.turns[1];
        s.nturns--;
    }
    uint16_t head = s.body[s.head];
    int nr = row(head) + DR[s.dir], nc = col(head) + DC[s.dir];
    if (!on_grid(nr, nc))
        return false;
    uint16_t next = cell(nr, nc);
    bool eating = next == s.food;
    if (occupied(next) && (eating || next != tail()))
        return false;
    if (!eating) {
        mark(tail(), false);
        s.len--;
    }
    push(next);
    if (eating) {
        s.score++;
        s.flash = 8;
        s.food = new_food();
    }
    return true;
}

bool snake_event(event_t ev, uint8_t arg)
{
    if (s.demo && ev == EV_TAP && arg == 3) {
        start(PLAY);
        return true;
    }
    if (s.state == PLAY && ev == EV_PRESS && (arg == 0 || arg == 3)) {
        if (s.nturns < 2)
            s.turns[s.nturns++] = arg == 0 ? -1 : 1;
        return true;
    }
    return !s.demo && ev == EV_TAP;
}

bool snake_playing(void)
{
    return !s.demo;
}

static void draw_number(uint16_t n, bool on)
{
    uint8_t digits[5];
    int len = 0;
    do {
        digits[len++] = n % 10;
        n /= 10;
    } while (n && len < 5);
    int c0 = (GRID_W - (len * 4 - 1)) / 2;
    for (int k = 0; k < len; k++) {
        const uint8_t *rows = font_digit3x5(digits[len - 1 - k]);
        for (int r = 0; r < 5; r++)
            for (int c = 0; c < 3; c++)
                if (rows[r] >> (2 - c) & 1)
                    fb_grid(5 + r, c0 + k * 4 + c, on ? 255 : 70);
    }
}

void snake_render(void)
{
    s.tick++;
    switch (s.state) {
    case ATTRACT:
    case PLAY: {
        uint32_t every = s.score >= 10 ? 3 : 4; /* 7.5 moves/s, 10 from a score of 10 */
        if (s.tick % every == 0 && !step()) {
            /* a game crashes; attract mode just pauses on its score */
            s.timer = s.state == PLAY ? CRASH_FRAMES : SCORE_FRAMES / 2;
            s.state = s.state == PLAY ? CRASH : SCORE;
        }
        break;
    }
    case CRASH:
        if (s.timer-- == 0) {
            s.state = SCORE;
            s.timer = SCORE_FRAMES;
        }
        break;
    case SCORE:
        if (s.timer-- == 0)
            start(ATTRACT);
        break;
    }

    if (s.state == SCORE) {
        draw_number(s.score, s.timer % 10 < 7);
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, hsv((uint8_t)(RGB[i].arm * 51 + RGB[i].rank * 18 - s.tick * 6), 255, 180));
        return;
    }

    bool crash = s.state == CRASH;
    if (!crash || s.timer % 6 < 3)
        for (int k = 0; k < s.len; k++) {
            uint16_t cl = s.body[(s.head + MAX_LEN - k) % MAX_LEN];
            int v = k == 0 ? 255 : 200 - k * 5 < 90 ? 90 : 200 - k * 5;
            fb_grid(row(cl), col(cl), (uint8_t)v);
        }
    if (!crash && s.tick % 6 < 4)
        fb_grid(row(s.food), col(s.food), 230);

    /* arms: score, 7 per arm; red while crashing */
    for (int i = 0; i < RGB_LEDS; i++) {
        int n = s.score - RGB[i].arm * 7;
        bool lit = (n > 7 ? 7 : n) > RGB[i].rank;
        rgb_t c = crash        ? rgb_scaled((rgb_t){255, 10, 0}, (uint8_t)(s.timer * (255 / CRASH_FRAMES)))
                  : s.flash    ? rgb_scaled((rgb_t){50, 255, 80}, (uint8_t)(s.flash * 31))
                  : lit        ? rgb_scaled((rgb_t){25, 255, 60}, 180)
                               : (rgb_t){0, 0, 0};
        fb_rgb(i, c);
    }
    /* outline: flash on eating; a slow dim chase while it plays itself */
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = s.flash ? (uint8_t)(s.flash * 25)
                      : s.state == ATTRACT && (j + s.tick / 3) % 12 == 0 ? 60
                                                                       : 0;
    if (s.flash)
        s.flash--;
}
