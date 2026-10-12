#include <string.h>
#include "anim.h"
#include "app.h"
#include "font.h"
#include "frame.h"
#include "ir.h"
#include "irproto.h"
#include "log.h"
#include "selftest.h"

enum { T_COM, T_PWM, T_LED, T_ALL, T_GREY, T_KEY, T_SCAN, T_IR, T_POWER, TESTS };
static const char *const NAME[TESTS] = {"COM", "PWM", "LED", "ALL", "GRY", "KEY", "SCN", "IR", "PWR"};
static const char *const BUTTON[4] = {"<", "B", "A", ">"}; /* ◀ B A ▶ in the log */
static const char *const KIND[] = {"press", "tap", "hold"};

#define LABEL_FRAMES FPS     /* the test's name shows this long first */
#define STEP_FRAMES (FPS / 2)
#define LED_FRAMES (FPS / 4)
#define IR_EVERY (2 * FPS)

static struct {
    uint8_t test;
    uint32_t t;      /* frames since this test started */
    int step, shown; /* walk position, and the one last logged */
    bool paused;
    uint32_t rows0, rows_t0, rate;
    uint16_t ir_sent, ir_heard;
    int vbus, chrg; /* last logged, -1 = not yet */
} s;

static void start_test(uint8_t test)
{
    s.test = test;
    s.t = 0;
    s.step = 0;
    s.shown = -1;
    s.paused = false;
    s.rows0 = ledpwm_rows();
    s.rows_t0 = 0;
    s.rate = 0;
    s.ir_sent = s.ir_heard = 0;
    s.vbus = s.chrg = -1;
    ir_loopback(test == T_IR);
    log_str("test ");
    log_u32(test + 1u);
    log_str("/");
    log_u32(TESTS);
    log_str(": ");
    log_line(NAME[test]);
}

void selftest_start(void)
{
    log_line("bring-up test: < > pick a test, hold A to leave");
    start_test(T_COM);
}

bool selftest_event(event_t ev, uint8_t arg)
{
    if (ev <= EV_HOLD && arg < 4) {
        log_str("key ");
        log_str(KIND[ev]);
        log_str(" ");
        log_line(BUTTON[arg]);
    }
    if (ev == EV_HOLD && arg == 2) {
        ir_loopback(false);
        log_line("bring-up test: done");
        return false;
    }
    if (ev == EV_TAP && arg == 0)
        start_test((uint8_t)((s.test + TESTS - 1) % TESTS));
    else if (ev == EV_TAP && arg == 3)
        start_test((uint8_t)((s.test + 1) % TESTS));
    else if (ev == EV_TAP && arg == 1 && s.test == T_LED)
        s.paused = !s.paused;
    else if (ev == EV_TAP && arg == 2 && s.test == T_LED && s.paused)
        s.step++;
    return true;
}

void selftest_ir(const uint8_t *data, int len)
{
    (void)data;
    (void)len;
    s.ir_heard++;
    log_str("ir: heard ");
    log_u32(s.ir_heard);
    log_str(" of ");
    log_u32(s.ir_sent);
    log_line(" sent");
}

static void log_slot(int slot)
{
    uint16_t ref = SLOT_REF[slot];
    log_str("slot ");
    log_u32((uint32_t)slot);
    log_str(": PB");
    log_u32((uint32_t)(slot / PWM_CHANNELS));
    log_str(" PA");
    log_u32((uint32_t)(slot % PWM_CHANNELS));
    if (!ref) {
        log_line(" no LED");
        return;
    }
    log_str(" D");
    log_u32(ref & 0x0FFFu);
    static const char *const COLOUR[4] = {"", " red", " green", " blue"};
    log_line(COLOUR[ref >> 12 & 3]);
}

static void walk(int every, int count)
{
    if (!s.paused && s.t % (uint32_t)every == 0 && s.t)
        s.step++;
    s.step %= count;
}

static bool logged_step(void)
{
    if (s.step == s.shown)
        return false;
    s.shown = s.step;
    return true;
}

static void draw_keys(uint8_t held)
{
    static const char *const GLYPH[4] = {"L", "B", "A", "R"};
    for (int i = 0; i < 4; i++)
        if (held >> i & 1)
            draw_text(GLYPH[i], 1 + i * 4, 4, 255);
}

void selftest_render(uint8_t held)
{
    uint32_t now = s.t++;
    if (now < LABEL_FRAMES && s.test != T_KEY) {
        draw_text(NAME[s.test], (GRID_W - text_width(NAME[s.test])) / 2, 4, 255);
        return;
    }
    switch (s.test) {
    case T_COM:
        walk(STEP_FRAMES, COMS);
        for (int p = 0; p < PWM_CHANNELS; p++)
            fb[s.step * PWM_CHANNELS + p] = 255;
        if (logged_step()) {
            log_str("COM PB");
            log_u32((uint32_t)s.step);
            log_line(" on");
        }
        break;
    case T_PWM:
        walk(STEP_FRAMES, PWM_CHANNELS);
        for (int c = 0; c < COMS; c++)
            fb[c * PWM_CHANNELS + s.step] = 255;
        if (logged_step()) {
            log_str("PWM PA");
            log_u32((uint32_t)s.step);
            log_line(" on");
        }
        break;
    case T_LED:
        walk(LED_FRAMES, SLOTS);
        fb[s.step] = 255;
        if (logged_step())
            log_slot(s.step);
        break;
    case T_ALL:
        memset(fb, 255, sizeof fb);
        if (now == LABEL_FRAMES) {
            log_str("all LEDs on, brightness level ");
            log_u32(app_level());
            log_line(" (0 night .. 4 full)");
        }
        break;
    case T_GREY:
        for (int r = 0; r < GRID_H; r++)
            for (int c = 0; c < GRID_W; c++)
                fb_grid(r, c, (uint8_t)(c * 255 / (GRID_W - 1)));
        for (int j = 0; j < EDGE_LEDS; j++)
            fb[EDGE[j]] = (uint8_t)(j * 255 / (EDGE_LEDS - 1));
        for (int i = 0; i < RGB_LEDS; i++) {
            static const rgb_t ARM[5] = {{255, 0, 0}, {0, 255, 0}, {0, 0, 255}, {255, 255, 255}, {255, 160, 0}};
            fb_rgb(i, rgb_scaled(ARM[RGB[i].arm], (uint8_t)(255 - RGB[i].rank * 30)));
        }
        if (now == LABEL_FRAMES)
            log_line("grid: grey ramp left to right; arms red, green, blue, white, orange");
        break;
    case T_KEY:
        draw_keys(held);
        break;
    case T_SCAN: {
        uint32_t rows = ledpwm_rows() - s.rows0;
        if (now - s.rows_t0 >= (uint32_t)FPS) {
            s.rate = rows * FPS / (now - s.rows_t0);
            s.rows0 += rows;
            s.rows_t0 = now;
            log_str("scan: ");
            log_u32(s.rate);
            log_line(" rows/s (2930 = 48 MHz LED clock, 183 Hz frames)");
        }
        draw_number((uint16_t)(s.rate > 65535 ? 65535 : s.rate), 5, 255);
        break;
    }
    case T_IR:
        if (now % IR_EVERY == 0 && ir_send(IR_TEST, (const uint8_t *)"TEST", 4)) {
            s.ir_sent++;
            log_str("ir: test packet ");
            log_u32(s.ir_sent);
            log_line(" sent");
        }
        draw_number(s.ir_heard, 2, 255);
        draw_number(s.ir_sent, 9, 80);
        break;
    case T_POWER: {
        int vbus = board_vbus(), chrg = app_charging();
        draw_text(vbus ? "USB" : "BAT", (GRID_W - text_width(vbus ? "USB" : "BAT")) / 2, 2, 255);
        if (chrg)
            draw_text("CHG", (GRID_W - text_width("CHG")) / 2, 9, 120);
        if (vbus != s.vbus || chrg != s.chrg) {
            s.vbus = vbus;
            s.chrg = chrg;
            log_str("power: ");
            log_str(vbus ? "USB (PC3 high)" : "battery (PC3 low)");
            log_line(chrg ? ", charging (CHRG low)" : ", not charging");
        }
        break;
    }
    }
}
