/* Higher or lower, same as ../../../src/games/hilo.rs: a number from 1 to
 * 99; will the next one be higher (▶) or lower (◀)? A right guess adds
 * one to the streak, which fills the outline; a wrong one ends the game.
 * Challenge 3 (challenge.c). */
#include <string.h>
#include "anim.h"
#include "font.h"
#include "games.h"

#define S (game_state.hilo)
#define SHOW_FRAMES 24
#define DEMO_THINK 20

enum { GUESS, SHOW };

static uint8_t pick(uint8_t not)
{
    uint8_t n;
    do
        n = (uint8_t)(1 + rng_next() % 99);
    while (n == not);
    return n;
}

void hilo_init(void)
{
    memset(&S, 0, sizeof S);
    S.now = pick(0);
    S.next = pick(S.now);
}

bool hilo_tick(bool demo, input_t in)
{
    S.t++;
    if (S.state == SHOW) {
        if (S.t < SHOW_FRAMES)
            return true;
        if (!S.right)
            return false;
        S.now = S.next;
        S.next = pick(S.now);
        S.state = GUESS;
        S.t = 0;
        return true;
    }
    int guess = 0; /* +1 higher, -1 lower */
    if (demo) {
        if (S.t >= DEMO_THINK)
            guess = S.now < 50 ? 1 : -1;
    } else if (in.pressed & BTN_RIGHT) {
        guess = 1;
    } else if (in.pressed & BTN_LEFT) {
        guess = -1;
    }
    if (!guess)
        return true;
    S.right = (S.next > S.now) == (guess > 0);
    if (S.right)
        S.streak++;
    S.state = SHOW;
    S.t = 0;
    return true;
}

void hilo_draw(uint32_t t)
{
    char s[3] = {0};
    uint8_t n = S.state == SHOW ? S.next : S.now;
    s[0] = (char)(n >= 10 ? '0' + n / 10 : '0' + n);
    s[1] = (char)(n >= 10 ? '0' + n % 10 : 0);
    draw_text(s, (GRID_W - text_width(s)) / 2, 4, 255);
    for (int j = 0; j < EDGE_LEDS; j++) /* the streak, 5 outline LEDs a point */
        fb[EDGE[j]] = j < S.streak * 5 ? 120 : 0;
    if (S.state == SHOW) {
        rgb_t c = S.right ? (rgb_t){0, 255, 60} : (rgb_t){255, 20, 0};
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, c);
    } else { /* ▶'s leg says higher, ◀'s lower */
        uint8_t blink = t % 30 < 20 ? 160 : 60;
        for (int k = 0; k < ARM_LINE; k++) {
            arm_line(BUTTON_ARM[3], k, rgb_scaled((rgb_t){0, 200, 80}, blink));
            arm_line(BUTTON_ARM[0], k, rgb_scaled((rgb_t){255, 60, 0}, blink));
        }
    }
}

uint16_t hilo_score(void)
{
    return S.streak;
}
