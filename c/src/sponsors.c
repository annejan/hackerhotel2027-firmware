#include "anim.h"
#include "font.h"
#include "frame.h"
#include "sponsors.h"

typedef struct {
    const char *name;
    const uint32_t *logo; /* GRID_H rows, or 0 */
} sponsor_t;

/* Logos: tools/sim/logo.py turns a picture into these rows. */
static const sponsor_t SPONSORS[] = {
    {"ALLNET.CHINA", 0},
    {"BADGE.TEAM", 0},
    {"NICOLAI ELECTRONICS", 0},
};
#define COUNT (int)(sizeof SPONSORS / sizeof SPONSORS[0])

#define SPEED 20          /* columns a second, twice the name tag's */
#define LOGO_FRAMES (5 * FPS / 2)
#define FADE (FPS / 2)

static struct {
    int seg;      /* -1: "THANKS TO", then the sponsors */
    bool on_logo; /* the logo, before the name */
    uint32_t t;
    uint32_t total_t; /* for the colours, over the whole reel */
} r;

static uint32_t scroll_frames(const char *s)
{
    return (uint32_t)(GRID_W + text_width(s)) * FPS / SPEED;
}

void sponsors_start(void)
{
    r.seg = -1;
    r.on_logo = false;
    r.t = r.total_t = 0;
}

static void begin(int seg)
{
    r.seg = seg;
    r.on_logo = seg < COUNT && SPONSORS[seg].logo;
    r.t = 0;
}

bool sponsors_render(void)
{
    if (r.seg >= COUNT)
        return false;
    uint32_t t = r.t++, total = r.total_t++;
    const char *text = r.seg < 0 ? "THANKS TO" : SPONSORS[r.seg].name;
    if (r.on_logo) {
        const uint32_t *logo = SPONSORS[r.seg].logo;
        uint8_t v = t < FADE ? (uint8_t)(t * 255 / FADE) : 255;
        for (int row = 0; row < GRID_H; row++)
            for (int c = 0; c < GRID_W; c++)
                if (logo[row] >> c & 1)
                    fb_grid(row, c, v);
        if (t + 1 >= LOGO_FRAMES) {
            r.on_logo = false;
            r.t = 0;
        }
    } else {
        draw_text(text, GRID_W - (int)(t * SPEED / FPS), 4, 255);
        if (t + 1 >= scroll_frames(text))
            begin(r.seg + 1);
    }
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, hsv((uint8_t)(RGB[i].arm * 51 + RGB[i].rank * 8 + total * 2), 255, 160));
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = (uint8_t)((j + total / 2) % 8 == 0 ? 160 : 10);
    return true;
}
