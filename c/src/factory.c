#include <string.h>
#include "anim.h"
#include "app.h"
#include "factory.h"
#include "font.h"
#include "frame.h"
#include "ir.h"
#include "irproto.h"
#include "log.h"
#include "selftest.h"

enum { F_LED, F_ROW, F_KEY, F_USB, F_IR, STEPS };
static const char *const NAME[STEPS] = {"LED", "ROW", "KEY", "USB", "IR"};
static const char *const HINT[STEPS] = {
    "all LEDs lit, arms red / green / blue? B = yes, A = no",
    "one line or one set of dots at a time, never two? B = yes, A = no",
    "press < B A > once each",
    "plug in USB",
    "listening for its own echo or another badge",
};
static const char *const BUTTON[4] = {"<", "B", "A", ">"};

#define LABEL FPS            /* the step's name shows this long first */
#define ROW_FRAMES 4         /* per line in ROW */
#define KEY_TIMEOUT (20 * FPS)
#define USB_TIMEOUT (10 * FPS)
#define USB_STEADY FPS
#define IR_EVERY FPS
#define IR_TIMEOUT (10 * FPS)
#define PASS_FRAMES (3 * FPS)

enum { RUNNING, PASS, FAIL };

static struct {
    uint8_t step, state;
    uint32_t t;      /* frames since the step (or the pass screen) started */
    uint8_t keys;    /* buttons seen in KEY, bit n = button n */
    uint32_t usb_t;  /* frames USB has been there without a break */
    uint16_t ir_sent, ir_heard;
} f;

static void start_step(uint8_t step)
{
    f.step = step;
    f.state = RUNNING;
    f.t = 0;
    f.keys = 0;
    f.usb_t = 0;
    f.ir_sent = f.ir_heard = 0;
    ir_loopback(step == F_IR);
    log_str("factory test ");
    log_u32(step + 1u);
    log_str("/");
    log_u32(STEPS);
    log_str(" ");
    log_str(NAME[step]);
    log_str(": ");
    log_line(HINT[step]);
}

static void next(void)
{
    if (f.step + 1 < STEPS) {
        start_step(f.step + 1);
        return;
    }
    ir_loopback(false);
    f.state = PASS;
    f.t = 0;
    log_line("factory test: PASS");
}

static void fail(const char *why)
{
    ir_loopback(false);
    f.state = FAIL;
    f.t = 0;
    log_str("factory test: FAIL at ");
    log_str(NAME[f.step]);
    log_str(": ");
    log_line(why);
    log_line("B tries the step again");
}

void factory_start(void)
{
    log_line("factory test: 5 steps, see docs/USAGE.md");
    start_step(F_LED);
}

void factory_event(event_t ev, uint8_t arg)
{
    if (f.state == FAIL) {
        if (ev == EV_TAP && arg == 1)
            start_step(f.step);
        return;
    }
    if (f.state != RUNNING || f.t < LABEL)
        return;
    if (f.step == F_KEY && ev == EV_PRESS && arg < 4) {
        f.keys |= (uint8_t)(1u << arg);
        log_str("key ");
        log_line(BUTTON[arg]);
        if (f.keys == 0xF)
            next();
    } else if ((f.step == F_LED || f.step == F_ROW) && ev == EV_TAP) {
        if (arg == 1)
            next();
        else if (arg == 2)
            fail("marked bad (A)");
    }
}

void factory_ir(void)
{
    if (f.state == RUNNING && f.step == F_IR) {
        f.ir_heard++;
        log_line("ir: test packet heard");
    }
}

static void draw_center(const char *s, int row, uint8_t v)
{
    draw_text(s, (GRID_W - text_width(s)) / 2, row, v);
}

static void arms(rgb_t c)
{
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, c);
}

/* the outline fills up as the steps go by */
static void progress(uint8_t v)
{
    for (int j = 0; j < EDGE_LEDS; j++)
        if (j * STEPS < (f.step + 1) * EDGE_LEDS)
            fb[EDGE[j]] = v;
}

static void step_render(uint32_t now)
{
    uint32_t t = now - LABEL;
    switch (f.step) {
    case F_LED: {
        static const rgb_t COLOUR[3] = {{255, 0, 0}, {0, 255, 0}, {0, 0, 255}};
        uint32_t p = t % (FPS * 9 / 2); /* 1.5 s white, then 1 s per colour */
        memset(fb, 255, sizeof fb);
        if (p >= FPS * 3 / 2)
            arms(COLOUR[(p - FPS * 3 / 2) / FPS]);
        break;
    }
    case F_ROW: {
        /* in the order you see them, not wiring order (row 7 is PB15,
         * column 7 is PA9): the COM line of grid row k top to bottom, then
         * the PWM channel of grid column k left to right, then PA17..PA23,
         * which only drive the outline and the arms. Column 8 has a LED in
         * every row, row 7 in every column. */
        int k = (int)(t / ROW_FRAMES % (COMS + PWM_CHANNELS));
        if (k < COMS) {
            int com = GRID[k][8] / PWM_CHANNELS;
            for (int p = 0; p < PWM_CHANNELS; p++)
                fb[com * PWM_CHANNELS + p] = 255;
        } else {
            k -= COMS;
            int pwm = k < GRID_W ? GRID[7][k] % PWM_CHANNELS : k;
            for (int c = 0; c < COMS; c++)
                fb[c * PWM_CHANNELS + pwm] = 255;
        }
        break;
    }
    case F_KEY:
        for (int i = 0; i < 4; i++)
            draw_text(BUTTON[i], i % 2 ? 10 : 2, i < 2 ? 2 : 9, f.keys >> i & 1 ? 255 : 25);
        progress(60);
        if (t >= KEY_TIMEOUT) {
            static char missing[] = "missing: < B A >";
            for (int i = 0; i < 4; i++)
                missing[9 + i * 2] = f.keys >> i & 1 ? '.' : BUTTON[i][0];
            fail(missing);
        }
        break;
    case F_USB: {
        bool vbus = board_vbus();
        f.usb_t = vbus ? f.usb_t + 1 : 0;
        draw_center("USB", 2, vbus || now / (FPS / 2) % 2 ? 255 : 30);
        if (app_charging())
            draw_center("CHG", 9, 120);
        progress(60);
        if (f.usb_t >= USB_STEADY)
            next();
        else if (t >= USB_TIMEOUT)
            fail("no USB power on PC3");
        break;
    }
    case F_IR:
        if (t % IR_EVERY == 0 && ir_send(IR_TEST, (const uint8_t *)"TEST", 4))
            f.ir_sent++;
        draw_number(f.ir_heard, 2, 255);
        draw_number(f.ir_sent, 9, 80);
        progress(60);
        if (f.ir_heard)
            next();
        else if (t >= IR_TIMEOUT)
            fail("nothing heard");
        break;
    }
}

factory_t factory_render(uint8_t held)
{
    (void)held;
    uint32_t now = f.t++;
    switch (f.state) {
    case PASS:
        arms((rgb_t){0, 255, 40});
        for (int j = 0; j < EDGE_LEDS; j++)
            fb[EDGE[j]] = 200;
        draw_center("OK", 4, 255);
        return now == 0 ? FACTORY_PASSED : now >= PASS_FRAMES ? FACTORY_DONE : FACTORY_BUSY;
    case FAIL:
        arms(now / (FPS / 2) % 2 ? (rgb_t){255, 0, 0} : (rgb_t){60, 0, 0});
        draw_center(NAME[f.step], 2, 255);
        draw_center("B", 9, 60);
        return FACTORY_BUSY;
    }
    if (now < LABEL) {
        draw_center(NAME[f.step], 4, 255);
        progress(60);
        return FACTORY_BUSY;
    }
    step_render(now);
    return FACTORY_BUSY;
}
