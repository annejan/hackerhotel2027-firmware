#include <string.h>
#include "anim.h"
#include "challenge.h"
#include "font.h"
#include "frame.h"

static const struct {
    game_kind_t game;
    const char *name;
    uint16_t level[CHALLENGE_LEVELS]; /* the score each level needs */
    rgb_t colour;
} CH[CHALLENGES] = {
    {GAME_MAZE, "maze", {1, 2, 3, 4, 5}, {60, 120, 255}},
    {GAME_SIMON, "simon", {4, 6, 8, 10, 12}, {255, 200, 0}},
    {GAME_WHACK, "whack-a-mole", {20, 40, 60, 80, 100}, {255, 40, 0}},
    {GAME_HILO, "higher lower", {3, 5, 7, 9, 12}, {170, 40, 255}},
    {GAME_SNAKE, "snake", {5, 10, 15, 20, 30}, {0, 255, 60}},
};

static uint8_t level[CHALLENGES];
static bool changed, victory_due;

void challenge_load(uint16_t packed)
{
    for (int c = 0; c < CHALLENGES; c++) {
        uint8_t l = packed >> (3 * c) & 7;
        level[c] = l > CHALLENGE_LEVELS ? CHALLENGE_LEVELS : l;
    }
}

uint16_t challenge_packed(void)
{
    uint16_t p = 0;
    for (int c = 0; c < CHALLENGES; c++)
        p |= (uint16_t)(level[c] << (3 * c));
    return p;
}

bool challenge_take_changed(void)
{
    bool c = changed;
    changed = false;
    return c;
}

uint8_t challenge_level(int c) { return level[c]; }
const char *challenge_name(int c) { return CH[c].name; }

int challenge_total(void)
{
    int n = 0;
    for (int c = 0; c < CHALLENGES; c++)
        n += level[c];
    return n;
}

int challenge_result(game_kind_t kind, uint16_t score)
{
    for (int c = 0; c < CHALLENGES; c++) {
        if (CH[c].game != kind)
            continue;
        uint8_t l = 0;
        while (l < CHALLENGE_LEVELS && score >= CH[c].level[l])
            l++;
        if (l <= level[c])
            return -1;
        level[c] = l;
        changed = true;
        if (challenge_total() == CHALLENGES * CHALLENGE_LEVELS)
            victory_due = true;
        return c;
    }
    return -1;
}

bool challenge_victory_due(void) { return victory_due; }
void challenge_victory_done(void) { victory_due = false; }

void challenge_arms(uint32_t t, int fresh)
{
    for (int c = 0; c < CHALLENGES; c++) {
        rgb_t col = CH[c].colour;
        for (int k = 0; k < ARM_LINE; k++) {
            uint8_t v = k >= level[c] ? 14 : c == fresh && k == level[c] - 1 && t % 12 < 6 ? 40 : 255;
            arm_line((uint8_t)c, k, rgb_scaled(col, v));
        }
        /* the two beside the line: bright, and shimmering, when it is complete */
        arm_sides((uint8_t)c, rgb_scaled(col, level[c] == CHALLENGE_LEVELS ? (uint8_t)(150 + sin8((uint8_t)(t * 6 + c * 51)) * 105 / 255) : 30));
    }
}

/* ---- the victory dance ----
 *
 * A stick figure dances on the grid, a new pose on every beat, dipping a
 * row at the start of each; the arms flash disco colours on the beat and
 * the outline chases. Between dances the text goes by a phrase at a time,
 * with fireworks on the arms and the outline. */

static const char *const PHRASE[] = {
    "I'VE COMPLETED", "ALL THE CHALLENGES", "AND ALL I'VE GOT", "IS THIS LOUSY", "LED ANIMATION!",
};
#define PHRASES (int)(sizeof PHRASE / sizeof PHRASE[0])
#define SPEED 24     /* text columns a second */
#define BEAT 12      /* frames a beat: 150 bpm */
#define DANCE_BEATS 8
#define ROCKETS 3
#define RISE 10      /* frames up the arm */
#define BURST 26     /* frames of the burst */

/* A pose: (row, column) of the neck, hip, left elbow and hand, right elbow
 * and hand, left knee and foot, right knee and foot ("left" as you look
 * at it). The head is a ring two rows above the neck. */
typedef struct {
    int8_t p[10][2];
} pose_t;

#define STAND {11, 7}, {14, 6}, {11, 9}, {14, 10}
static const pose_t POSES[] = {
    /* 0 arms up (and Y) */ {{{4, 8}, {9, 8}, {3, 6}, {1, 4}, {3, 10}, {1, 12}, STAND}},
    /* 1 arms down */ {{{4, 8}, {9, 8}, {6, 6}, {9, 5}, {6, 10}, {9, 11}, STAND}},
    /* 2 point up right, left hand on the hip */
    {{{4, 8}, {9, 8}, {6, 5}, {8, 7}, {3, 11}, {1, 13}, {11, 7}, {14, 5}, {11, 9}, {14, 10}}},
    /* 3 point up left, right hand on the hip */
    {{{4, 8}, {9, 8}, {3, 5}, {1, 3}, {6, 11}, {8, 9}, {11, 7}, {14, 6}, {11, 9}, {14, 11}}},
    /* 4 arms out */ {{{4, 8}, {9, 8}, {5, 5}, {4, 2}, {5, 11}, {4, 14}, STAND}},
    /* 5 squat, arms forward */
    {{{5, 8}, {10, 8}, {6, 5}, {5, 3}, {6, 11}, {5, 13}, {11, 5}, {14, 5}, {11, 11}, {14, 11}}},
    /* 6 jump: arms up, knees tucked */
    {{{3, 8}, {8, 8}, {2, 6}, {0, 7}, {2, 10}, {0, 9}, {10, 6}, {12, 7}, {10, 10}, {12, 9}}},
    /* 7 M: hands on the head */ {{{4, 8}, {9, 8}, {3, 5}, {1, 6}, {3, 11}, {1, 10}, STAND}},
    /* 8 C: both arms curved to the left */
    {{{4, 8}, {9, 8}, {3, 6}, {2, 4}, {6, 7}, {7, 4}, STAND}},
    /* 9 A: hands meet above the head */ {{{4, 8}, {9, 8}, {2, 6}, {0, 7}, {2, 10}, {0, 9}, STAND}},
    /* 10 kick right */
    {{{4, 8}, {9, 8}, {5, 5}, {3, 3}, {6, 10}, {8, 11}, {11, 8}, {14, 7}, {10, 10}, {9, 13}}},
    /* 11 kick left */
    {{{4, 8}, {9, 8}, {6, 6}, {8, 5}, {5, 11}, {3, 13}, {10, 6}, {9, 3}, {11, 8}, {14, 9}}},
};

/* the moves, a pose a beat, round and round */
static const uint8_t STEPS[] = {0, 1, 0, 1, 2, 3, 2, 3, 4, 5, 4, 6, 0, 7, 8, 9, 10, 11, 10, 11, 5, 6, 4, 1};
#define STEPS_N (int)(sizeof STEPS)

typedef struct {
    uint8_t arm, hue, t; /* t = 0: free */
} rocket_t;

static struct {
    uint32_t t;      /* frames into this part */
    int part;        /* even: a dance, odd: a phrase; the last dance ends it */
    uint32_t beat;   /* beats danced, over all the dances */
    rocket_t rocket[ROCKETS];
} v;

void victory_start(void)
{
    memset(&v, 0, sizeof v);
}

static uint8_t tip_angle(uint8_t arm)
{
    for (int i = 0; i < RGB_LEDS; i++)
        if (RGB[i].arm == arm && RGB[i].rank == 6)
            return angle(RGB[i].r);
    return 0;
}

static void fireworks(void)
{
    for (int j = 0; j < EDGE_LEDS; j++) /* a faint twinkle everywhere */
        fb[EDGE[j]] = rng_byte() < 12 ? 90 : 6;
    for (int k = 0; k < ROCKETS; k++) {
        rocket_t *r = &v.rocket[k];
        if (!r->t) {
            if (rng_byte() < 24) { /* a new one */
                r->arm = (uint8_t)(rng_next() % 5);
                r->hue = rng_byte();
                r->t = 1;
            }
            continue;
        }
        if (r->t <= RISE) { /* a spark climbs the arm, with a short trail */
            int at = (r->t - 1) * ARM_LINE / RISE;
            arm_line(r->arm, at, (rgb_t){255, 230, 180});
            if (at > 0)
                arm_line(r->arm, at - 1, (rgb_t){80, 50, 20});
        } else { /* and bursts: the arm and the outline by its tip, fading */
            uint8_t fade = (uint8_t)(255 - (r->t - RISE) * 255 / BURST);
            rgb_t c = hsv(r->hue, 230, fade);
            for (int i = 0; i < RGB_LEDS; i++)
                if (RGB[i].arm == r->arm && rng_byte() < 200)
                    fb_rgb(i, c);
            uint8_t tip = tip_angle(r->arm);
            for (int j = 0; j < EDGE_LEDS; j++) {
                int d = (int8_t)(uint8_t)(angle(EDGE[j]) - tip);
                if (d > -14 && d < 14 && rng_byte() < 150)
                    fb[EDGE[j]] = fade;
            }
        }
        if (++r->t > RISE + BURST)
            r->t = 0;
    }
}

static void line(int r0, int c0, int r1, int c1)
{
    int dr = absi(r1 - r0), dc = absi(c1 - c0), sr = r0 < r1 ? 1 : -1, sc = c0 < c1 ? 1 : -1;
    int err = dc - dr;
    for (;;) {
        fb_grid(r0, c0, 255);
        if (r0 == r1 && c0 == c1)
            return;
        int e2 = 2 * err;
        if (e2 > -dr) {
            err -= dr;
            c0 += sc;
        }
        if (e2 < dc) {
            err += dc;
            r0 += sr;
        }
    }
}

static void dancer(const pose_t *p, int dy)
{
    const int8_t(*q)[2] = p->p;
#define AT(k) q[k][0] + dy, q[k][1]
    line(AT(0), AT(1));                 /* body */
    line(AT(0), AT(2)), line(AT(2), AT(3)); /* arms */
    line(AT(0), AT(4)), line(AT(4), AT(5));
    line(AT(1), AT(6)), line(AT(6), AT(7)); /* legs */
    line(AT(1), AT(8)), line(AT(8), AT(9));
#undef AT
    int hr = q[0][0] + dy - 2, hc = q[0][1]; /* the head: a ring */
    for (int dr = -1; dr <= 1; dr++)
        for (int dc = -1; dc <= 1; dc++)
            if (dr || dc)
                fb_grid(hr + dr, hc + dc, 255);
}

static void dance(uint32_t t)
{
    uint32_t in_beat = t % BEAT, beat = v.beat + t / BEAT;
    uint8_t pose = STEPS[beat % STEPS_N];
    /* dip at the start of a beat; the jump goes up instead */
    dancer(&POSES[pose], pose == 6 ? (in_beat < BEAT / 2 ? -1 : 0) : in_beat < 3);
    /* disco: the arms flash a new colour every beat, the outline chases */
    uint8_t flash = (uint8_t)(255 - in_beat * 160 / BEAT);
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, hsv((uint8_t)(beat * 47 + RGB[i].arm * 20), 255, flash));
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = (uint8_t)((j + beat) % 2 ? 140 : 8);
}

bool victory_render(void)
{
    uint32_t t = v.t++;
    if (v.part % 2 == 0) { /* dancing */
        dance(t);
        if (t + 1 >= DANCE_BEATS * BEAT) {
            v.beat += DANCE_BEATS;
            v.part++;
            v.t = 0;
        }
        return v.part <= 2 * PHRASES;
    }
    /* a phrase goes by; its letters bob, a little after each other */
    const char *s = PHRASE[v.part / 2];
    int len = (int)strlen(s);
    int x0 = GRID_W - (int)(t * SPEED / FPS);
    for (int i = 0; i < len; i++) {
        int x = x0 + i * (FONT_W + 1);
        if (x <= -FONT_W || x >= GRID_W || s[i] == ' ')
            continue;
        char ch[2] = {s[i], 0};
        draw_text(ch, x, 4 + (sin8((uint8_t)(t * 12 - i * 40)) - 128) * 2 / 128, 255);
    }
    fireworks();
    if (x0 + len * (FONT_W + 1) <= 0) {
        v.part++;
        v.t = 0;
    }
    return true;
}
