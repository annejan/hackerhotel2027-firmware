/* Plays the challenge games for real through the app, reading their state
 * to press the right buttons: every challenge reaches level 5, the
 * progress packs and loads back, a worse score never lowers a level, and
 * the victory dance comes at 25 and keeps going from the VICTORY item. */
#include <string.h>
#include "app.h"
#include "challenge.h"
#include "check.h"
#include "events.h"
#include "frame.h"
#include "games.h"

static void press(int b) { app_event(EV_PRESS, (uint8_t)b); app_event(EV_TAP, (uint8_t)b); }
static void frame(void) { app_render(0); }
static void frames(int n) { while (n--) frame(); }

static int item_of(const char *label)
{
    for (int i = 0; i < app_items(); i++)
        if (strcmp(app_label((uint8_t)i), label) == 0)
            return i;
    return -1;
}
static void start(const char *label) { app_set_item((uint8_t)item_of(label)); frames(3); press(0); frame(); }

static void play_hilo(void)
{
    start("HIGHER LOWER");
    for (int i = 0; i < 4000 && game_playing(); i++) {
        hilo_t *h = &game_state.hilo;
        if (h->state == 0 && h->t > 3)
            press(h->next > h->now ? 3 : 0);
        frame();
        if (h->streak >= 13) /* then lose on purpose */
            while (game_playing()) { press(h->next > h->now ? 0 : 3); frames(30); }
    }
}

static void play_simon(void)
{
    start("SIMON");
    for (int i = 0; i < 60000 && game_playing(); i++) {
        simon_t *s = &game_state.simon;
        if (s->state == 1 && s->idle > 3) {
            if (s->len > 12 && s->pos == 0) { press((s->seq[0] + 1) % 4); frames(40); continue; }
            press(s->seq[s->pos]);
        }
        frame();
    }
}

static void play_whack(void)
{
    start("WHACK-A-MOLE");
    for (int i = 0; i < 200000 && game_playing(); i++) {
        whack_t *w = &game_state.whack;
        if (w->state == 1 && w->lit == 1 && w->score < 110)
            press(1);
        frame();
    }
}

static const int8_t DR[4] = {0, 1, 0, -1}, DC[4] = {1, 0, -1, 0};
static const int KEY[4] = {3, 2, 0, 1}; /* direction -> button: right ▶, down A, left ◀, up B */
static void play_maze(void)
{
    start("MAZE");
    int dir = 0;
    for (int i = 0; i < 200000 && game_playing(); i++) {
        maze_t *m = &game_state.maze;
        if (!m->fx && i % 3 == 0)
            for (int k = 0; k < 4; k++) { /* right-hand wall follower */
                int d = (dir + 1 + 3 * k) % 4;
                int r = m->r + DR[d], c = m->c + DC[d];
                if (!(m->wall[r] >> c & 1)) { dir = d; press(KEY[d]); break; }
            }
        frame();
    }
}

int main(void)
{
    name_t n;
    name_default(&n);
    app_init(&n);
    challenge_load(0);
    play_hilo();
    CHECK(challenge_level(3) == 5, "higher lower: level 5 (got %d)", challenge_level(3));
    play_simon();
    CHECK(challenge_level(1) == 5, "simon: level 5 (got %d)", challenge_level(1));
    play_whack();
    CHECK(challenge_level(2) == 5, "whack-a-mole: level 5 (got %d)", challenge_level(2));
    play_maze();
    CHECK(challenge_level(0) == 5, "maze: all five solved (got %d)", challenge_level(0));
    CHECK(challenge_total() == 20 && !challenge_victory_due(), "20 of 25, no victory yet");
    CHECK(challenge_take_changed() && !challenge_take_changed(), "a change is handed to main.c once");
    challenge_result(GAME_SNAKE, 30); /* snake: just its score */
    CHECK(challenge_victory_due(), "victory due at 25");
    frames(200);
    CHECK(!challenge_victory_due(), "the dance started once the game was over");
    uint16_t p = challenge_packed();
    challenge_load(0);
    CHECK(challenge_total() == 0, "load(0) clears");
    challenge_load(p);
    CHECK(challenge_total() == 25, "packed progress loads back (0x%04x)", p);
    challenge_load(0xFFFF);
    CHECK(challenge_total() <= 25, "an out-of-range packed word does not go past 25 (%d)", challenge_total());
    challenge_load(p);
    CHECK(challenge_result(GAME_HILO, 1) == -1 && challenge_total() == 25, "a worse score never lowers a level");
    frames(1200); /* the whole dance, then the name tag */
    app_set_item((uint8_t)item_of("VICTORY"));
    frames(3000);
    CHECK(app_item() == item_of("VICTORY"), "the VICTORY item keeps dancing");
    TEST_END("challenge");
}
