/* Shared game shell, same as ../../src/game.rs. */
#include "anim.h"
#include "game.h"
#include "games.h"

#define CRASH_FRAMES 20
#define SCORE_FRAMES (2 * FPS)

union game_state game_state;

typedef struct {
    void (*init)(void);
    bool (*tick)(bool demo, input_t in);
    void (*draw)(uint32_t t);
    uint16_t (*score)(void);
    bool (*won)(void); /* the player won: no red crash; NULL = never */
    bool uses_edge;    /* draws on the outline itself: no demo-mode chase */
} game_ops_t;

#define OPS(name) name##_init, name##_tick, name##_draw, name##_score
static const game_ops_t OPS_TABLE[GAMES] = {
    [GAME_SNAKE] = {OPS(snake), 0, false},
    [GAME_STARTRIS] = {OPS(startris), 0, false},
    [GAME_BREAKOUT] = {OPS(breakout), 0, false},
    [GAME_PONG] = {OPS(pong), pong_won, false},
    [GAME_FLAPPY] = {OPS(flappy), 0, false},
    [GAME_CHASE] = {OPS(chase), 0, true},
};

enum { RUN, CRASH, SCORE };

static struct {
    const game_ops_t *ops;
    uint8_t kind, state, timer, pressed;
    bool demo;
    uint32_t t;
} g;

static void restart(bool demo)
{
    g.ops->init();
    g.state = RUN;
    g.timer = 0;
    g.demo = demo;
    g.pressed = 0;
    g.t = 0;
}

void game_start(game_kind_t kind)
{
    g.kind = (uint8_t)kind;
    g.ops = &OPS_TABLE[kind];
    restart(true);
}

bool game_playing(void)
{
    return !g.demo;
}

bool game_event(event_t ev, uint8_t arg)
{
    switch (ev) {
    case EV_PRESS:
        if (g.demo) {
            if (arg == 2)
                return false; /* T3: tap = brightness, hold = menu */
            restart(false);
            return true;
        }
        g.pressed |= (uint8_t)(1u << arg);
        return true;
    case EV_HOLD:
        if (arg == 2)
            return false; /* menu, even from a game */
        return !g.demo;
    case EV_TAP:
        return !g.demo;
    default:
        return false;
    }
}

void game_render(uint8_t held)
{
    g.t++;
    input_t in = {0, 0};
    if (!g.demo) {
        in.pressed = g.pressed;
        in.held = held;
        g.pressed = 0;
    }
    switch (g.state) {
    case RUN:
        if (!g.ops->tick(g.demo, in)) {
            bool won = g.ops->won && g.ops->won();
            g.state = g.demo || won ? SCORE : CRASH;
            g.timer = g.demo ? SCORE_FRAMES / 2 : won ? SCORE_FRAMES : CRASH_FRAMES;
        }
        break;
    case CRASH:
        if (g.timer == 0) {
            g.state = SCORE;
            g.timer = SCORE_FRAMES;
        } else {
            g.timer--;
        }
        break;
    case SCORE:
        if (g.timer == 0) {
            restart(true);
            return;
        }
        g.timer--;
        break;
    }

    switch (g.state) {
    case SCORE:
        draw_number(g.ops->score(), 5, g.timer % 10 < 7 ? 255 : 70);
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, hsv((uint8_t)(RGB[i].arm * 51 + RGB[i].rank * 18 - g.t * 6), 255, 180));
        break;
    case CRASH:
        if (g.timer % 6 < 3)
            g.ops->draw(g.t);
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, rgb_scaled((rgb_t){255, 10, 0}, (uint8_t)(g.timer * (255 / CRASH_FRAMES))));
        break;
    default:
        /* demo mode: a slow dim chase round the outline says "touch me" */
        if (g.demo && !g.ops->uses_edge)
            for (int j = 0; j < EDGE_LEDS; j++)
                if ((j + g.t / 3) % 12 == 0)
                    fb[EDGE[j]] = 60;
        g.ops->draw(g.t);
        break;
    }
}

void arm_fill(uint8_t arm, int n, rgb_t c)
{
    for (int i = 0; i < RGB_LEDS; i++)
        if (RGB[i].arm == arm && RGB[i].rank < n)
            fb_rgb(i, c);
}

void arms_score(uint16_t score, rgb_t c)
{
    for (uint8_t a = 0; a < 5; a++)
        arm_fill(a, clampi(score - a * 7, 0, 7), c);
}

int approach(int *pos, int target, uint32_t t, uint32_t every)
{
    if (t % every != 0 || *pos == target)
        return 0;
    int d = target > *pos ? 1 : -1;
    *pos += d;
    return d;
}
