/* games/four.c: the rules (four across, up and both diagonals, a draw),
 * the badge's play (take a win, block one), and the infrared side of a
 * game on one badge: invitations, joining, the other badge's moves,
 * answers, and the link timing out. Includes four.c itself, to reach its
 * state and helpers; the game shell (game.c) runs it as on the badge.
 * tools/emu/ir_game.py plays whole games between two emulated badges. */
#include <string.h>
#include "check.h"
#include "stubs.h"
#include "../src/games/four.c"

static void fresh(void)
{
    four_init();
    S.mode = LOCAL;
}

static void play(const char *cols)
{
    for (; *cols; cols++)
        drop(*cols - '0');
}

static int win_cells(void)
{
    int n = 0;
    for (int r = 0; r < FOUR_H; r++)
        for (int c = 0; c < FOUR_W; c++)
            n += !!(S.cell[r][c] & WIN_BIT);
    return n;
}

/* a packet from the other badge, through the shell */
static void rx(uint8_t game, uint8_t n, uint8_t col, uint8_t flags)
{
    uint8_t d[4] = {game, n, col, flags};
    game_ir(d, 4);
}

static int frames(int n)
{
    int sends = stub_sends;
    for (int i = 0; i < n; i++)
        game_render(0);
    return stub_sends - sends;
}

int main(void)
{
    frame_init();
    static const char *const WINS[] = {"0011223", "0101010", "01122323363", "65544343303"};
    for (unsigned i = 0; i < sizeof WINS / sizeof WINS[0]; i++) {
        fresh();
        play(WINS[i]);
        CHECK(S.winner == 1 && win_cells() == 4, "%s: player 1 wins with four marked (winner %d, %d marked)",
              WINS[i], S.winner, win_cells());
    }
    fresh();
    play("001122");
    CHECK(!S.winner, "three across is not yet a win");

    fresh();
    play("06162");
    CHECK(ai(2) == 3, "the badge blocks 0 1 2");
    drop(5);
    CHECK(ai(1) == 3, "and takes 0 1 2 3");

    /* a full board without four in a row, but for the last disc */
    static const uint8_t FULL[FOUR_H][FOUR_W] = {
        {1, 1, 2, 1, 2, 1, 0}, {2, 2, 1, 2, 2, 1, 1}, {2, 2, 2, 1, 1, 2, 1},
        {2, 1, 2, 1, 2, 1, 1}, {1, 2, 2, 1, 1, 2, 2}, {1, 2, 1, 2, 1, 1, 2},
    };
    fresh();
    memcpy(S.cell, FULL, sizeof FULL);
    S.n = FOUR_W * FOUR_H - 1;
    drop(6);
    CHECK(S.winner == 3, "a full board is a draw");

    /* infrared, through the game shell */
    game_start(GAME_FOUR);
    uint8_t ours = S.game;
    rx(ours, 3, 2, 0);
    CHECK(!game_playing(), "the demo ignores anything but an invitation");
    log_clear();
    rx(0xFF, 0, 0, FL_ANSWER);
    CHECK(game_playing() && S.mode == LINK && S.me == 1 && S.game == 0xFF, "an invitation starts a game and joins it");
    CHECK(strstr(stub_log, "four: linked, they start") != NULL, "and logs it [%s]", stub_log);
    CHECK(frames(10) == 1 && stub_sent[0] == IR_FOUR && stub_sent[2] == 0xFF && stub_sent[3] == 0 &&
              stub_sent[5] == FL_SECOND,
          "it answers once, as the second player (%02x %02x %02x %02x)", stub_sent[2], stub_sent[3], stub_sent[4],
          stub_sent[5]);
    rx(0xFF, 1, 3, FL_ANSWER);
    CHECK(S.n == 1 && S.cell[FOUR_H - 1][3] == 1, "their first move lands");
    rx(0xFF, 1, 3, FL_ANSWER);
    rx(0xFF, 1, 4, FL_ANSWER);
    CHECK(S.n == 1, "the same move again changes nothing");
    rx(0x12, 2, 4, FL_ANSWER);
    rx(0xFF, 2, 4, 0);
    CHECK(S.n == 1, "nor does another game's move, or a move that is not theirs to make");
    CHECK(frames(10) == 1, "one answer for all of those");

    /* our move: sent until they answer */
    game_event(EV_PRESS, 1);
    frames(1);
    CHECK(S.n == 2 && S.cell[FOUR_H - 2][3] == 2, "B drops ours on theirs");
    CHECK(frames(1) == 1 && stub_sent[3] == 2 && stub_sent[4] == 3 && stub_sent[5] == (FL_ANSWER | FL_SECOND),
          "and sends it the next frame, asking for an answer");
    CHECK(frames(2 * FPS) >= 2, "again while they do not answer");
    rx(0xFF, 2, 3, 0);
    CHECK(S.heard == 2, "their answer");
    int pings = frames(10 * FPS);
    CHECK(pings >= 2 && pings <= 4, "then only a question every 3 s while it is their turn (%d)", pings);

    /* nothing heard for 20 s: the link is lost */
    log_clear();
    frames(21 * FPS);
    CHECK(strstr(stub_log, "four: link lost") != NULL && !game_playing(), "20 s of silence ends it [%s]", stub_log);

    /* both invite: the higher number asks, the lower one joins */
    game_start(GAME_FOUR);
    game_event(EV_PRESS, 0);
    S.game = 0x80;
    rx(0x40, 0, 0, FL_ANSWER);
    CHECK(S.mode == SEEK && S.game == 0x80, "a lower number's invitation is not taken");
    rx(0x80, 0, 0, FL_ANSWER);
    CHECK(S.mode == SEEK && S.game != 0x80, "the same number: pick another");
    uint8_t g = S.game;
    rx(g, 0, 0, FL_SECOND);
    CHECK(S.mode == LINK && S.me == 0, "someone joined ours: we start");
    /* they lose */
    log_clear();
    for (int i = 0; i < 7; i++) {
        if (i % 2 == 0) {
            S.cursor = 0;
            game_event(EV_PRESS, 1);
        } else {
            rx(g, (uint8_t)(i + 1), 6, FL_ANSWER | FL_SECOND);
        }
        frames(1);
    }
    CHECK(S.winner == 1 && strstr(stub_log, "four: you win, moves 1717171") != NULL, "four up wins [%s]", stub_log);
    CHECK(four_won() && four_score() == 17, "and scores the discs left in hand (%d)", four_score());
    TEST_END("four");
}
