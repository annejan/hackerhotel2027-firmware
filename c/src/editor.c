/* Name entry on the badge (EDIT NAME in the menu), same as
 * ../../src/editor.rs.
 *
 *   ◀ / ▶   previous / next character (hold to keep going)
 *   tap A    next position; past the end it wraps to the start
 *   tap B    previous position
 *   hold A   save and show the name tag
 * Left alone for a minute, it closes without saving.
 *
 * The grid shows the name with the character being edited in the middle,
 * bright, with a blinking underline; the outline shows the position. */
#include "anim.h"
#include "editor.h"
#include "frame.h"
#include "game.h"

/* the characters ◀ / ▶ step through, in this order; from a blank, ▶
 * gives A */
static const char CHARS[] = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-.!?'*:/#@";
#define NCHARS ((int)sizeof CHARS - 1)
#define TIMEOUT (60 * FPS)
#define REPEAT_AFTER 12 /* frames before a held button repeats */
#define REPEAT_EVERY 4

static struct {
    char buf[NAME_MAX];
    int cursor;
    uint8_t rep[2]; /* frames ◀ / ▶ held */
    uint32_t idle, t;
} e;

void editor_start(const name_t *name)
{
    for (int i = 0; i < NAME_MAX; i++)
        e.buf[i] = i < name->len ? name->s[i] : ' ';
    e.cursor = 0;
    e.rep[0] = e.rep[1] = 0;
    e.idle = e.t = 0;
}

static void step(int d)
{
    int i = 0;
    while (i < NCHARS && CHARS[i] != e.buf[e.cursor])
        i++;
    if (i == NCHARS)
        i = 0;
    e.buf[e.cursor] = CHARS[wrapi(i + d, NCHARS)];
}

/* one past the last character that is not a space */
static int end(void)
{
    int n = NAME_MAX;
    while (n > 0 && e.buf[n - 1] == ' ')
        n--;
    return n;
}

edit_t editor_event(event_t ev, uint8_t arg, name_t *out)
{
    e.idle = 0;
    if (ev == EV_PRESS && arg == 0) {
        step(-1);
        e.rep[0] = 0;
    } else if (ev == EV_PRESS && arg == 3) {
        step(1);
        e.rep[1] = 0;
    } else if (ev == EV_TAP && arg == 2) {
        /* one further, up to one blank after the name, then back to the start */
        int last = end() < NAME_MAX - 1 ? end() : NAME_MAX - 1;
        e.cursor = e.cursor + 1 > last ? 0 : e.cursor + 1;
    } else if (ev == EV_TAP && arg == 1) {
        if (e.cursor > 0)
            e.cursor--;
    } else if (ev == EV_HOLD && arg == 2) {
        name_set(out, e.buf, NAME_MAX);
        return EDIT_SAVE;
    }
    return EDIT_BUSY;
}

edit_t editor_render(uint8_t held)
{
    e.t++;
    if (++e.idle > TIMEOUT)
        return EDIT_CANCEL;
    static const uint8_t BTN[2] = {BTN_LEFT, BTN_RIGHT};
    for (int k = 0; k < 2; k++) {
        if (held & BTN[k]) {
            if (e.rep[k] < 255)
                e.rep[k]++;
            if (e.rep[k] > REPEAT_AFTER && e.rep[k] % REPEAT_EVERY == 0)
                step(k ? 1 : -1);
        } else {
            e.rep[k] = 0;
        }
    }

    /* the name, with the cursor's character at columns 6..10 */
    const int c0 = (GRID_W - 5) / 2;
    for (int i = 0; i < NAME_MAX; i++) {
        int x = c0 + (i - e.cursor) * 6;
        if (x > -6 && x < GRID_W) {
            char s[2] = {e.buf[i], 0};
            draw_text(s, x, 3, i == e.cursor ? 255 : 60);
        }
    }
    uint8_t blink = e.t % 20 < 12 ? 255 : 40;
    for (int c = 0; c < 5; c++)
        fb_grid(11, c0 + c, blink);
    /* the outline marks the position in the name */
    int lo = e.cursor * EDGE_LEDS / NAME_MAX, hi = (e.cursor + 1) * EDGE_LEDS / NAME_MAX;
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = j >= lo && j < hi ? 220 : 12;
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, rgb_scaled((rgb_t){40, 200, 255}, 70));
    return EDIT_BUSY;
}
