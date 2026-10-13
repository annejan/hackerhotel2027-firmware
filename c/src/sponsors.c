#include "anim.h"
#include "font.h"
#include "frame.h"
#include "logos.h"
#include "sponsors.h"

typedef struct {
    const char *text;     /* scrolls after the logo; 0 when the logo says it */
    const uint32_t *logo; /* logos.h, or 0 */
    int logo_w;
    rgb_t arm[2];         /* the arms alternate between these */
} sponsor_t;

#define LOGO(name) LOGO_##name, (int)(sizeof LOGO_##name / sizeof LOGO_##name[0])

/* colours from the logos, brightened for LEDs */
static const sponsor_t SPONSORS[] = {
    {"ALLNET.CHINA", LOGO(ALLNET), {{130, 178, 255}, {130, 178, 255}}},
    {0, LOGO(BADGETEAM), {{255, 255, 255}, {255, 255, 255}}},
    {0, LOGO(NICOLAI), {{240, 67, 0}, {70, 70, 62}}},
};
#define COUNT (int)(sizeof SPONSORS / sizeof SPONSORS[0])

#define SPEED 20          /* columns a second, twice the name tag's */
#define LOGO_FRAMES (5 * FPS / 2) /* a logo that fits stands this long */
#define FADE (FPS / 2)

static struct {
    int seg;      /* -1: "THANKS TO", then the sponsors */
    bool on_logo; /* the logo, before the text */
    uint32_t t;
    uint32_t total_t; /* for the colours, over the whole reel */
} r;

static uint32_t scroll_frames(int w)
{
    return (uint32_t)(GRID_W + w) * FPS / SPEED;
}

static void begin(int seg)
{
    r.seg = seg;
    r.on_logo = seg >= 0 && seg < COUNT && SPONSORS[seg].logo;
    r.t = 0;
}

void sponsors_start(void)
{
    begin(-1);
    r.total_t = 0;
}

/* greyscale logo, column 0 at grid column x */
static void draw_logo(const sponsor_t *s, int x, uint8_t v)
{
    for (int c = 0; c < s->logo_w; c++) {
        int gx = x + c;
        if (gx < 0 || gx >= GRID_W)
            continue;
        for (int row = 0; row < GRID_H; row++) {
            uint32_t level = s->logo[c] >> (2 * row) & 3;
            if (level)
                fb_grid(row, gx, scale8((uint8_t)(level * 85), v));
        }
    }
}

/* the logo, standing in the middle or scrolling by; true when it is over */
static bool logo_frame(const sponsor_t *s, uint32_t t)
{
    if (s->logo_w <= GRID_W) {
        draw_logo(s, (GRID_W - s->logo_w) / 2, t < FADE ? (uint8_t)(t * 255 / FADE) : 255);
        return t + 1 >= LOGO_FRAMES;
    }
    draw_logo(s, GRID_W - (int)(t * SPEED / FPS), 255);
    return t + 1 >= scroll_frames(s->logo_w);
}

bool sponsors_render(void)
{
    if (r.seg >= COUNT)
        return false;
    uint32_t t = r.t++, total = r.total_t++;
    const sponsor_t *s = r.seg >= 0 ? &SPONSORS[r.seg] : 0;
    if (r.on_logo) {
        if (logo_frame(s, t)) {
            r.on_logo = false;
            r.t = 0;
            if (!s->text)
                begin(r.seg + 1);
        }
    } else {
        const char *text = s ? s->text : "THANKS TO";
        draw_text(text, GRID_W - (int)(t * SPEED / FPS), 4, 255);
        if (t + 1 >= scroll_frames(text_width(text)))
            begin(r.seg + 1);
    }
    for (int i = 0; i < RGB_LEDS; i++) {
        if (s) /* a wave of the sponsor's colours runs out along the arms */
            fb_rgb(i, rgb_scaled(s->arm[RGB[i].arm % 2], (uint8_t)(90 + sin8((uint8_t)(RGB[i].rank * 36 - total * 8)) * 165 / 255)));
        else
            fb_rgb(i, hsv((uint8_t)(RGB[i].arm * 51 + RGB[i].rank * 8 + total * 2), 255, 160));
    }
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = (uint8_t)((j + total / 2) % 8 == 0 ? 160 : 10);
    return true;
}
