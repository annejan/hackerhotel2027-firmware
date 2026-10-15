/* Four in a row, same as ../../../src/games/four.rs: drop discs into seven
 * columns, six high; four of yours in a line (across, up or diagonal)
 * wins. ◀ / ▶ aim, B drops.
 *
 * Against another badge over infrared, or against this one. A new game
 * looks for another badge first: point the two badges at each other with
 * 4 IN A ROW open on both, and they pair up. Dropping a disc before that
 * plays against the badge instead.
 *
 * Your discs are bright, the other side's dim. The arms are green on your
 * turn and orange on theirs, and sweep blue while looking for a badge.
 *
 * Infrared: packet IR_FOUR, data [game, move, column, flags]. `game` is a
 * random number that keeps two pairs of players apart. A badge looking for
 * a partner sends move 0 with FL_ANSWER every second or so; a badge with
 * the game open answers and joins, and the one that asked moves first
 * (when both ask, the higher number asks). The one that joined sets
 * FL_SECOND in everything it sends. After that each move goes out
 * as soon as it is made, again every 0.8 s until the other badge answers.
 * An answer is our own last move, so whichever packet arrives, both
 * badges end up with the same board. While it is the other badge's turn,
 * ours asks every 3 s whether it is still there; 20 s without an answer
 * ends the game. Every game is logged on the UART with its moves. */
#include <string.h>
#include "anim.h"
#include "font.h"
#include "games.h"
#include "ir.h"
#include "irproto.h"
#include "log.h"

#define S (game_state.four)
#define X0 2 /* column c is grid column X0 + 2c */
#define Y0 5 /* row r (0 = top) is grid row Y0 + r */
#define CURSOR_Y 3
#define FL_ANSWER 1           /* please answer: we have not heard you got this */
#define FL_SECOND 2           /* the sender moves second */
#define WIN_BIT 4             /* cell is part of the winning line */
#define DEMO_EVERY 24         /* frames per demo move */
#define BADGE_THINK 15        /* frames before the badge answers your move */
#define RESEND (FPS * 4 / 5)  /* resend an unanswered move this often, plus jitter */
#define PING (3 * FPS)        /* their turn: ask whether they are still there */
#define LOST (20 * FPS)       /* no answer for this long: the link is gone */
#define END_FRAMES (3 * FPS)  /* show the result this long */

enum { SEEK, LOCAL, LINK };

static uint8_t row_free(int c) /* the row a disc dropped in column c lands in, or 0xFF */
{
    for (int r = FOUR_H - 1; r >= 0; r--)
        if (!S.cell[r][c])
            return (uint8_t)r;
    return 0xFF;
}

static int owner(int r, int c)
{
    return (unsigned)r < FOUR_H && (unsigned)c < FOUR_W ? S.cell[r][c] & 3 : 0;
}

/* discs of player p (1 or 2) through (r, c) in direction (dr, dc), (r, c)
 * counted as p's; mark = also mark them as the winning line */
static int line(int r, int c, int dr, int dc, int p, bool mark)
{
    int n = 1;
    for (int s = -1; s <= 1; s += 2)
        for (int k = 1; owner(r + s * k * dr, c + s * k * dc) == p; k++) {
            n++;
            if (mark)
                S.cell[r + s * k * dr][c + s * k * dc] |= WIN_BIT;
        }
    return n;
}

static const int8_t DIRS[4][2] = {{0, 1}, {1, 0}, {1, 1}, {1, -1}};

static bool wins(int r, int c, int p)
{
    for (int d = 0; d < 4; d++)
        if (line(r, c, DIRS[d][0], DIRS[d][1], p, false) >= 4)
            return true;
    return false;
}

static bool wins_in(int c, int p)
{
    uint8_t r = row_free(c);
    return r != 0xFF && wins(r, c, p);
}

/* the badge's move for player p: win, block, keep away from the edges, and
 * do not set up the other side's win on top */
static int ai(int p)
{
    for (int k = 0; k < 2; k++)
        for (int c = 0; c < FOUR_W; c++)
            if (wins_in(c, k ? 3 - p : p))
                return c;
    int best = -1, best_v = -1000;
    for (int c = 0; c < FOUR_W; c++) {
        uint8_t r = row_free(c);
        if (r == 0xFF)
            continue;
        int v = (3 - absi(c - 3)) * 4 + (int)(rng_next() % 4);
        S.cell[r][c] = (uint8_t)p;
        if (r > 0 && wins(r - 1, c, 3 - p))
            v -= 50;
        S.cell[r][c] = 0;
        if (v > best_v) {
            best_v = v;
            best = c;
        }
    }
    return best;
}

static void result(void)
{
    if (!game_playing())
        return;
    log_str(S.winner == 3 ? "four: draw" : S.winner == S.me + 1 ? "four: you win" : "four: you lose");
    log_str(", moves ");
    char m[FOUR_W * FOUR_H + 1];
    for (int i = 0; i < S.n; i++)
        m[i] = (char)('1' + S.moves[i]);
    m[S.n] = 0;
    log_line(m);
}

/* the next move, in column c (it fits) */
static void drop(int c)
{
    int p = S.n % 2 + 1, r = row_free(c);
    S.cell[r][c] = (uint8_t)p;
    S.moves[S.n++] = (uint8_t)c;
    S.last = (uint8_t)(r * FOUR_W + c);
    S.since = 0;
    bool won = false;
    for (int d = 0; d < 4; d++)
        if (line(r, c, DIRS[d][0], DIRS[d][1], p, false) >= 4) {
            line(r, c, DIRS[d][0], DIRS[d][1], p, true);
            won = true;
        }
    if (won) {
        S.cell[r][c] |= WIN_BIT;
        S.winner = (uint8_t)p;
    } else if (S.n == FOUR_W * FOUR_H) {
        S.winner = 3;
    }
    if (S.winner)
        result();
}

static bool my_turn(void)
{
    return S.n % 2 == S.me;
}

static void send(bool answer)
{
    uint8_t d[4] = {S.game, S.n, S.n ? S.moves[S.n - 1] : 0, (uint8_t)((answer ? FL_ANSWER : 0) | (S.me ? FL_SECOND : 0))};
    if (!ir_send(IR_FOUR, d, 4))
        S.answer_in = 1; /* still sending: try again next frame */
}

void four_init(void)
{
    memset(&S, 0, sizeof S);
    S.cursor = 3;
    S.game = (uint8_t)(1 + rng_next() % 255);
    S.resend_in = 1;
}

bool four_ir(bool live, const uint8_t *d, int len)
{
    if (len != 4)
        return false;
    uint8_t game = d[0], n = d[1], c = d[2];
    bool asks = d[3] & FL_ANSWER, second = d[3] & FL_SECOND;
    bool invite = n == 0 && asks && !second;
    if (!live)
        return invite; /* the shell starts a game */
    if (S.mode == SEEK) {
        if (invite && game > S.game) { /* join theirs; they move first */
            S.game = game;
            S.me = 1;
        } else if (n == 0 && game == S.game && second) {
            S.me = 0; /* they joined ours */
        } else {
            if (invite && game == S.game) /* the same number: pick another */
                S.game = (uint8_t)(1 + rng_next() % 255);
            return false;
        }
        S.mode = LINK;
        S.resend_in = PING;
        log_line(S.me ? "four: linked, they start" : "four: linked, you start");
    } else if (S.mode != LINK || game != S.game || second == S.me) {
        return false; /* not our game, or not the other player */
    }
    S.quiet = 0;
    if (n == S.n + 1 && !S.winner && S.n % 2 != S.me && c < FOUR_W && row_free(c) != 0xFF)
        drop(c);
    if (n <= S.n && n > S.heard)
        S.heard = n;
    if (asks && !S.answer_in)
        S.answer_in = (uint8_t)(3 + rng_next() % 6); /* let their receiver settle */
    return false;
}

bool four_tick(bool demo, input_t in)
{
    if (S.since < 0xFFFF)
        S.since++;
    if (S.answer_in && --S.answer_in == 0)
        send(S.n > S.heard);
    if (!demo && S.mode == SEEK && --S.resend_in == 0) {
        S.resend_in = (uint16_t)(FPS + rng_next() % 20);
        send(true);
    }
    /* they have not got our last move, or it is their turn: keep asking */
    if (S.mode == LINK && (S.n > S.heard || (!my_turn() && !S.winner))) {
        if (++S.quiet > LOST) {
            log_line("four: link lost");
            return false;
        }
        if (--S.resend_in == 0) {
            S.resend_in = (uint16_t)((S.n > S.heard ? RESEND : PING) + rng_next() % 12);
            send(true);
        }
    }
    if (S.winner)
        return ++S.end < END_FRAMES;

    if (demo) {
        if (S.since >= DEMO_EVERY)
            drop(S.cursor = (uint8_t)ai(S.n % 2 + 1));
        return true;
    }
    if (in.pressed & BTN_LEFT && S.cursor > 0)
        S.cursor--;
    if (in.pressed & BTN_RIGHT && S.cursor < FOUR_W - 1)
        S.cursor++;
    if (S.mode == LOCAL && S.n % 2 == 1) { /* the badge's turn */
        if (S.since >= BADGE_THINK)
            drop(ai(2));
    } else if (in.pressed & BTN_B && my_turn() && row_free(S.cursor) != 0xFF) {
        if (S.mode == SEEK)
            S.mode = LOCAL;
        drop(S.cursor);
        if (S.mode == LINK) {
            S.quiet = 0;
            S.resend_in = 1;
        }
    }
    return true;
}

void four_draw(uint32_t t)
{
    bool demo = !game_playing();
    for (int x = 1; x < GRID_W - 1; x++) /* the floor */
        fb_grid(Y0 + FOUR_H, x, 20);
    for (int r = 0; r < FOUR_H; r++)
        for (int c = 0; c < FOUR_W; c++) {
            uint8_t v = S.cell[r][c];
            if (!v)
                continue;
            uint8_t b = (v & 3) == (demo ? 1 : S.me + 1) ? 255 : 60;
            if (v & WIN_BIT && t % 10 < 5)
                b = 0;
            else if (S.n && r * FOUR_W + c == S.last && S.since < FPS && S.since % 8 < 4)
                b = 0; /* the last disc blinks a little */
            fb_grid(Y0 + r, X0 + 2 * c, b);
        }
    if (!S.winner && !demo && my_turn())
        fb_grid(CURSOR_Y, X0 + 2 * S.cursor, t % 16 < 10 ? 255 : 90);

    rgb_t c;
    if (S.winner)
        c = S.winner == 3 ? (rgb_t){200, 200, 200} : demo || S.winner == S.me + 1 ? (rgb_t){0, 255, 60} : (rgb_t){255, 20, 0};
    else if (S.mode == SEEK && !demo) { /* looking for a badge: a blue wave runs out */
        for (int i = 0; i < RGB_LEDS; i++)
            if ((int)((t / 3) % 9) - RGB[i].rank == 0)
                fb_rgb(i, (rgb_t){0, 80, 255});
        return;
    } else
        c = demo ? S.n % 2 ? (rgb_t){255, 90, 0} : (rgb_t){0, 255, 60} : my_turn() ? (rgb_t){0, 255, 60} : (rgb_t){255, 90, 0};
    uint8_t pulse = (uint8_t)(120 + sin8((uint8_t)(t * 4)) / 2);
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, rgb_scaled(c, pulse));
}

uint16_t four_score(void)
{
    /* discs left in hand when you won */
    return four_won() ? (uint16_t)(FOUR_W * FOUR_H / 2 - (S.n + 1 - S.me) / 2) : 0;
}

bool four_won(void)
{
    return S.winner && S.winner == S.me + 1 && S.mode != SEEK;
}
