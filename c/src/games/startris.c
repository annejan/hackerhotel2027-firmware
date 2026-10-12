/* Startris, same as ../../../src/games/startris.rs: a 9-wide, 14-deep well
 * in the middle of the octagon. ◀ left, ▶ right (repeat when held), A
 * rotate, B drop. The arms show the next piece's colour and fill with
 * cleared lines. The demo places each piece where a simple
 * height/holes/bumpiness score likes it best. */
#include <limits.h>
#include <string.h>
#include "games.h"

#define S (game_state.startris)
#define W 9
#define H WELL_H
#define R0 1 /* well top row on the grid */
#define C0 4 /* well left column on the grid */
#define FULL ((1u << W) - 1)
#define PIECE_I 0
#define PIECE_O 1

/* cells of each piece in its 3x3 box (I: 4x4); rotation turns the box */
static const int8_t SHAPES[7][4][2] = {
    {{1, 0}, {1, 1}, {1, 2}, {1, 3}}, /* I */
    {{0, 0}, {0, 1}, {1, 0}, {1, 1}}, /* O */
    {{0, 1}, {1, 0}, {1, 1}, {1, 2}}, /* T */
    {{0, 1}, {0, 2}, {1, 0}, {1, 1}}, /* S */
    {{0, 0}, {0, 1}, {1, 1}, {1, 2}}, /* Z */
    {{0, 0}, {1, 0}, {1, 1}, {1, 2}}, /* J */
    {{0, 2}, {1, 0}, {1, 1}, {1, 2}}, /* L */
};
static const uint8_t HUE[7] = {128, 40, 200, 85, 0, 165, 20};

static void cells(uint8_t kind, uint8_t rot, int out[4][2])
{
    int n = kind == PIECE_I ? 4 : 3;
    for (int i = 0; i < 4; i++) {
        int r = SHAPES[kind][i][0], c = SHAPES[kind][i][1];
        if (kind != PIECE_O)
            for (int k = 0; k < rot % 4; k++) {
                int t = r;
                r = c;
                c = n - 1 - t;
            }
        out[i][0] = r;
        out[i][1] = c;
    }
}

static uint8_t deal(void)
{
    if (S.bag == 0x7F)
        S.bag = 0;
    for (;;) {
        uint8_t k = (uint8_t)(rng_next() % 7);
        if (!(S.bag & 1u << k)) {
            S.bag |= (uint8_t)(1u << k);
            return k;
        }
    }
}

static bool fits_in(const uint16_t *well, uint8_t kind, uint8_t rot, int r, int c)
{
    int cl[4][2];
    cells(kind, rot, cl);
    for (int i = 0; i < 4; i++) {
        int rr = r + cl[i][0], cc = c + cl[i][1];
        if (rr < 0 || rr >= H || cc < 0 || cc >= W || well[rr] >> cc & 1)
            return false;
    }
    return true;
}

static bool fits(uint8_t rot, int r, int c)
{
    return fits_in(S.well, S.kind, rot, r, c);
}

static void place(uint16_t *well, uint8_t kind, uint8_t rot, int r, int c)
{
    int cl[4][2];
    cells(kind, rot, cl);
    for (int i = 0; i < 4; i++)
        well[r + cl[i][0]] |= (uint16_t)(1u << (c + cl[i][1]));
}

/* remove the rows in mask, letting the rest fall; returns how many were removed */
static int compact(uint16_t *well, uint16_t mask)
{
    int j = H;
    for (int i = H - 1; i >= 0; i--)
        if (!(mask >> i & 1))
            well[--j] = well[i];
    for (int i = 0; i < j; i++)
        well[i] = 0;
    return j;
}

static uint16_t full_rows(const uint16_t *well)
{
    uint16_t m = 0;
    for (int i = 0; i < H; i++)
        if (well[i] == FULL)
            m |= (uint16_t)(1u << i);
    return m;
}

/* how much the demo likes dropping the piece at (rot, c) */
static int rate(uint8_t rot, int c)
{
    int r = 0;
    while (fits(rot, r + 1, c))
        r++;
    uint16_t w[H];
    memcpy(w, S.well, sizeof w);
    place(w, S.kind, rot, r, c);
    int lines = compact(w, full_rows(w));
    int agg = 0, holes = 0, bump = 0, prev = -1;
    for (int x = 0; x < W; x++) {
        int top = 0;
        while (top < H && !(w[top] >> x & 1))
            top++;
        int height = H - top;
        for (int i = top; i < H; i++)
            holes += !(w[i] >> x & 1);
        agg += height;
        if (prev >= 0)
            bump += absi(height - prev);
        prev = height;
    }
    return 76 * lines - 51 * agg - 36 * holes - 18 * bump;
}

static bool spawn(void)
{
    S.kind = S.next;
    S.next = deal();
    S.rot = 0;
    S.r = 0;
    S.c = 3;
    if (!fits(0, 0, 3))
        return false;
    int best = INT_MIN;
    S.target_rot = 0;
    S.target_c = 3;
    for (uint8_t rot = 0; rot < (S.kind == PIECE_O ? 1 : 4); rot++)
        for (int c = -2; c < W; c++)
            if (fits(rot, 0, c)) {
                int v = rate(rot, c);
                if (v > best) {
                    best = v;
                    S.target_rot = rot;
                    S.target_c = c;
                }
            }
    return true;
}

static void rotate(void)
{
    uint8_t rot = (uint8_t)((S.rot + 1) % 4);
    static const int8_t KICKS[3] = {0, -1, 1};
    for (int k = 0; k < 3; k++)
        if (fits(rot, S.r, S.c + KICKS[k])) {
            S.rot = rot;
            S.c += KICKS[k];
            return;
        }
}

static void shift(int d)
{
    if (fits(S.rot, S.r, S.c + d))
        S.c += d;
}

/* put the piece down; false when the next one has no room */
static bool lock(void)
{
    place(S.well, S.kind, S.rot, S.r, S.c);
    S.clearing = full_rows(S.well);
    if (S.clearing) {
        S.clear_t = 9;
        return true;
    }
    return spawn();
}

static bool drop(void)
{
    while (fits(S.rot, S.r + 1, S.c))
        S.r++;
    return lock();
}

void startris_init(void)
{
    memset(&S, 0, sizeof S);
    S.c = 3;
    S.next = deal();
    spawn();
}

bool startris_tick(bool demo, input_t in)
{
    if (S.clear_t) {
        if (--S.clear_t == 0) {
            compact(S.well, S.clearing);
            S.lines += (uint16_t)popcount32(S.clearing);
            S.clearing = 0;
            return spawn();
        }
        return true;
    }
    if (demo) {
        if (++S.fall % 4 == 0) {
            /* if the wanted rotation or move is blocked, drop where it is */
            int rot = S.rot, c = S.c;
            if (S.rot != S.target_rot)
                rotate();
            else if (S.c != S.target_c)
                shift(S.target_c > S.c ? 1 : -1);
            if (S.rot == rot && S.c == c)
                return drop();
        }
        return true;
    }
    if (in.pressed & BTN_A)
        rotate();
    static const uint8_t BTN[2] = {BTN_LEFT, BTN_RIGHT};
    for (int k = 0; k < 2; k++) {
        int d = k ? 1 : -1;
        if (in.pressed & BTN[k]) {
            shift(d);
            S.rep[k] = 0;
        } else if (in.held & BTN[k]) {
            if (S.rep[k] < 255)
                S.rep[k]++;
            if (S.rep[k] > 8 && S.rep[k] % 3 == 0)
                shift(d);
        }
    }
    if (in.pressed & BTN_B)
        return drop();
    uint32_t speed = S.lines >= 16 ? 4 : 20u - S.lines;
    if (++S.fall >= speed) {
        S.fall = 0;
        if (fits(S.rot, S.r + 1, S.c))
            S.r++;
        else
            return lock();
    }
    return true;
}

void startris_draw(uint32_t t)
{
    for (int r = 0; r < H; r++) {
        fb_grid(R0 + r, C0 - 1, 25); /* well walls */
        fb_grid(R0 + r, C0 + W, 25);
        bool blink = S.clearing >> r & 1;
        for (int c = 0; c < W; c++)
            if (S.well[r] >> c & 1)
                fb_grid(R0 + r, C0 + c, !blink ? 170 : S.clear_t % 3 < 2 ? 255 : 40);
    }
    if (!S.clear_t) {
        int cl[4][2];
        cells(S.kind, S.rot, cl);
        for (int i = 0; i < 4; i++)
            fb_grid(R0 + S.r + cl[i][0], C0 + S.c + cl[i][1], 255);
    }
    rgb_t next = hsv(HUE[S.next], 255, 255);
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, rgb_scaled(next, 40));
    arms_score(S.lines, rgb_scaled(next, 220));
    if (S.clear_t)
        for (int i = 0; i < RGB_LEDS; i++)
            fb_rgb(i, hsv((uint8_t)(RGB[i].arm * 51 + RGB[i].rank * 9 + t * 12), 255, 255));
}

uint16_t startris_score(void)
{
    return S.lines;
}
