/* The app through its public interface (app.h, console.h, proto.h), with
 * stubs.c for the hardware: names, EDIT NAME, the infrared greeting, the
 * UART console, the factory test, the USB protocol, label scrolling.
 * Menu positions are looked up by label, so adding an item does not
 * break these. */
#include <stdlib.h>
#include <string.h>
#include "anim.h"
#include "app.h"
#include "check.h"
#include "console.h"
#include "font.h"
#include "frame.h"
#include "proto.h"
#include "stubs.h"

enum { L, B, A, R };

static void tap(int b) { app_event(EV_PRESS, (uint8_t)b); app_event(EV_TAP, (uint8_t)b); }
static void frames(int n, uint8_t held) { while (n--) app_render(held); }

static int item_of(const char *label)
{
    for (int i = 0; i < app_items(); i++)
        if (strcmp(app_label((uint8_t)i), label) == 0)
            return i;
    printf("no menu item %s\n", label);
    exit(2);
}

static void init_name(const char *s)
{
    name_t n;
    name_set(&n, s, (int)strlen(s));
    app_init(&n);
    stage_t st;
    while (app_stage(&st)) /* forget what earlier checks left */
        ;
}

/* hold A, ▶ to EDIT NAME (the menu opens on the current item, NAME), tap A */
static void open_editor(void)
{
    app_event(EV_HOLD, A);
    for (int i = 0; i < item_of("EDIT NAME"); i++)
        app_event(EV_TAP, R);
    app_event(EV_TAP, A);
}

static bool saved_is(const char *want)
{
    name_t n;
    return app_saved_name(&n) && strcmp(n.s, want) == 0;
}

static void names(void)
{
    name_t n;
    name_set(&n, "anne jan  ", 10);
    CHECK(strcmp(n.s, "ANNE JAN") == 0 && n.len == 8, "upper case, trailing blanks dropped (\"%s\")", n.s);
    name_set(&n, "", 0);
    CHECK(n.s[0] == 0 && n.len == 0, "empty name");
    name_set(&n, "a~b\x01", 4);
    CHECK(strcmp(n.s, "A B") == 0, "characters outside the font become blanks (\"%s\")", n.s);
    char x[40];
    memset(x, 'x', sizeof x);
    name_set(&n, x, 40);
    CHECK(n.len == NAME_MAX && strlen(n.s) == NAME_MAX, "cut to NAME_MAX");
}

static void editor(void)
{
    name_t n;
    init_name("AB");
    open_editor();
    frames(2, 0);
    app_event(EV_PRESS, R); /* A -> B */
    app_event(EV_TAP, A);   /* next position */
    app_event(EV_PRESS, L); /* B -> A */
    app_event(EV_TAP, A);
    app_event(EV_PRESS, L); /* blank -> @ (wraps backwards) */
    app_event(EV_HOLD, A);  /* save */
    CHECK(saved_is("BA@"), "edit and save: BA@");
    CHECK(!app_saved_name(&n), "a saved name is handed out once");

    init_name("HI");
    open_editor();
    for (int i = 0; i < 3; i++)
        app_event(EV_TAP, A); /* H -> I -> blank -> back to H */
    app_event(EV_PRESS, R);   /* H -> I */
    app_event(EV_HOLD, A);
    CHECK(saved_is("II"), "the position wraps one blank after the name: II");

    init_name("HI");
    open_editor();
    app_event(EV_TAP, A);   /* to I */
    app_event(EV_TAP, B);   /* B: back to H */
    app_event(EV_PRESS, R); /* H -> I */
    app_event(EV_HOLD, A);
    CHECK(saved_is("II"), "B goes to the previous position");

    init_name("A");
    open_editor();
    app_event(EV_PRESS, R); /* A -> B on touch-down */
    frames(40, 1u << R);    /* then 7 repeats */
    app_event(EV_HOLD, A);
    CHECK(saved_is("I"), "a held ▶ repeats: I");

    init_name("KEEP");
    open_editor();
    app_event(EV_PRESS, R);
    frames(61 * FPS, 0);
    CHECK(!app_saved_name(&n), "an idle editor closes after a minute without saving");
    app_event(EV_HOLD, A);
    CHECK(!app_saved_name(&n), "and hold A opens the menu then, not a save");
}

static void infrared(void)
{
    init_name("ME");
    frames(2, 0);
    stub_sends = 0;
    app_event(EV_TAP, B);
    CHECK(stub_sends == 1 && stub_sent[0] == IR_HELLO && stub_sent[1] == 2 && !memcmp(stub_sent + 2, "ME", 2),
          "B on the name tag sends HELLO ME");
    stub_ir_busy = true;
    CHECK(!app_ir_hello(), "no second send while busy");
    stub_ir_busy = false;
    stub_sends = 0;
    app_ir_received(IR_HELLO, (const uint8_t *)"you", 3);
    frames(12, 0);
    CHECK(stub_sends == 1 && stub_sent[0] == IR_REPLY && !memcmp(stub_sent + 2, "ME", 2), "HELLO is answered with REPLY ME");
    stub_sends = 0;
    app_ir_received(IR_REPLY, (const uint8_t *)"YOU", 3);
    frames(12, 0);
    CHECK(stub_sends == 0, "REPLY is not answered");
    stub_ir_busy = true;
    app_ir_received(IR_HELLO, (const uint8_t *)"X", 1);
    frames(30, 0);
    CHECK(stub_sends == 0, "the reply waits while the LED is busy");
    stub_ir_busy = false;
    frames(2, 0);
    CHECK(stub_sends == 1 && stub_sent[0] == IR_REPLY, "then goes out");
    app_ir_received('Z', (const uint8_t *)"X", 1);
    app_ir_received(IR_HELLO, (const uint8_t *)"", 0);
    app_ir_received(IR_HELLO, (const uint8_t *)"   ", 3);
    stub_sends = 0;
    frames(12, 0);
    CHECK(stub_sends == 0, "unknown types and empty names are ignored");

    uint8_t before[SLOTS], during[SLOTS];
    frames(5 * FPS, 0);
    app_render(0);
    memcpy(before, fb, sizeof fb);
    app_ir_received(IR_REPLY, (const uint8_t *)"Q", 1);
    app_render(0);
    memcpy(during, fb, sizeof fb);
    CHECK(memcmp(before, during, sizeof fb) != 0, "a greeting shows");
    app_event(EV_TAP, L);
    CHECK(app_item() == 0, "a tap only closes the greeting");

    /* a greeting waits while the menu is open */
    app_event(EV_HOLD, A);
    app_ir_received(IR_REPLY, (const uint8_t *)"Q", 1);
    app_render(0);
    memcpy(during, fb, sizeof fb);
    app_event(EV_HOLD, A); /* close the menu */
    frames(1, 0);
    app_render(0);
    CHECK(memcmp(fb, during, sizeof fb) != 0, "the greeting shows after the menu closed");
    app_event(EV_TAP, L);
}

static bool logged(const char *s) { return strstr(stub_log, s) != NULL; }

static void console(void)
{
    init_name("ME");
    log_clear();
    console_line("help");
    CHECK(logged("  0 NAME") && logged("  18 4 IN A ROW") && logged("  21 LIGHT") && !logged("  22 "), "help lists the 22 items");
    log_clear();
    console_line("  info");
    CHECK(logged("c firmware, name \"ME\", showing 0 NAME, level 2"), "info, after leading blanks [%s]", stub_log);

    log_clear();
    console_line("show 3");
    CHECK(app_item() == 3 && logged("showing 3 GALAXY"), "show 3 [%s]", stub_log);
    static const char *const bad[] = {"show", "show x", "show 3x", "show -1", "show 22", "show 99999999999",
                                      "level 5", "level", "factoryskip", "showx 1", "", "HELP"};
    for (unsigned i = 0; i < sizeof bad / sizeof bad[0]; i++) {
        log_clear();
        console_line(bad[i]);
        CHECK(logged("? try help") && app_item() == 3, "\"%s\" is refused [%s]", bad[i], stub_log);
    }
    char cmd[16];
    snprintf(cmd, sizeof cmd, "show %d", item_of("EDIT NAME"));
    log_clear();
    console_line(cmd);
    CHECK(logged("? try help") && app_item() == 3, "show EDIT NAME is refused");
    snprintf(cmd, sizeof cmd, "show %d", item_of("LIGHT"));
    log_clear();
    console_line(cmd);
    CHECK(logged("? try help") && app_item() == 3, "show LIGHT is refused");

    console_line("level 4");
    CHECK(app_level() == 4 && app_brightness() == 255, "level 4 is full brightness");
    console_line("level 0");
    CHECK(app_level() == 0, "level 0");

    name_t n;
    log_clear();
    console_line("name  bob the builder");
    CHECK(app_saved_name(&n) && strcmp(n.s, "BOB THE BUILDER") == 0, "name TEXT sets and saves (\"%s\")", n.s);

    log_clear();
    console_line("progress");
    CHECK(logged("total 0/25"), "progress [%s]", stub_log);

    stub_ir_busy = true;
    log_clear();
    console_line("hello");
    CHECK(logged("ir: busy"), "hello while sending says busy");
    stub_ir_busy = false;
    stub_sends = 0;
    console_line("hello");
    CHECK(stub_sends == 1 && stub_sent[0] == IR_HELLO, "hello sends");

    stage_t st;
    console_line("factory skip");
    CHECK(app_stage(&st) && st == STAGE_TESTED, "factory skip marks the test passed");
}

/* run the factory test for n frames; the IR stub echoes test packets
 * while loopback is on, as the badge's receiver hears its own LED */
static void factory_frames(int n, bool echo)
{
    while (n--) {
        int before = stub_sends;
        app_render(0);
        if (echo && stub_ir_loop && stub_sends != before && stub_sent[0] == IR_TEST)
            app_ir_received(IR_TEST, stub_sent + 2, stub_sent[1]);
    }
}

/* a fresh factory test, through LED, ROW, KEY and USB to the IR step */
static void to_ir(void)
{
    stub_vbus = true;
    app_factory();
    for (int i = 0; i < 2; i++) {
        factory_frames(FPS + 1, false);
        tap(B);
    }
    factory_frames(FPS + 1, false);
    for (int b = 0; b < 4; b++)
        app_event(EV_PRESS, (uint8_t)b);
    factory_frames(FPS + FPS + 2, false);
    CHECK(logged("5/5 IR"), "on to the IR step");
    log_clear();
}

static void factory(void)
{
    stage_t st;
    init_name("ME");
    log_clear();
    app_factory();
    factory_frames(FPS / 2, false);
    tap(B); /* during the step's name: ignored */
    factory_frames(FPS, false);
    CHECK(logged("1/5 LED") && !logged("2/5"), "taps during the step's name are ignored");
    tap(B);
    factory_frames(FPS + 1, false);
    tap(B);
    factory_frames(FPS + 1, false);
    CHECK(logged("3/5 KEY"), "LED and ROW pass on B [%s]", stub_log);
    for (int b = 0; b < 4; b++)
        app_event(EV_PRESS, (uint8_t)b);
    CHECK(logged("4/5 USB"), "KEY passes when all four were pressed");
    stub_vbus = true;
    factory_frames(FPS + FPS + 2, false);
    CHECK(logged("5/5 IR"), "USB passes after a second of USB power");
    CHECK(stub_ir_loop, "IR loopback on during the IR step");
    factory_frames(2 * FPS + 2, true);
    CHECK(logged("factory test: PASS"), "IR passes on its own echo [%s]", stub_log);
    CHECK(!stub_ir_loop, "loopback off after the test");
    CHECK(app_stage(&st) && st == STAGE_TESTED, "a pass is handed to main.c as STAGE_TESTED");
    factory_frames(3 * FPS + 2, false);
    CHECK(app_item() == 0, "after the pass screen: the name tag");
    tap(R);
    CHECK(app_item() == 1, "and the badge reacts to buttons again");

    /* failures */
    log_clear();
    stub_vbus = false;
    app_factory();
    factory_frames(FPS + 1, false);
    tap(A);
    CHECK(logged("factory test: FAIL at LED: marked bad (A)"), "A in LED fails it [%s]", stub_log);
    tap(B);
    factory_frames(FPS + 1, false);
    tap(B);
    factory_frames(FPS + 1, false);
    tap(B);
    factory_frames(FPS + 1, false);
    app_event(EV_PRESS, B);
    app_event(EV_PRESS, R);
    factory_frames(20 * FPS + 1, false);
    CHECK(logged("factory test: FAIL at KEY: missing: < . A ."), "KEY times out after 20 s, naming the missing buttons [%s]",
          stub_log);
    CHECK(!app_stage(&st), "a failed test hands out no stage");
    tap(B); /* retry KEY */
    factory_frames(FPS + 1, false);
    for (int b = 0; b < 4; b++)
        app_event(EV_PRESS, (uint8_t)b);
    factory_frames(10 * FPS + FPS + 2, false);
    CHECK(logged("FAIL at USB: no USB power on PC3"), "USB fails after 10 s without power");
    app_event(EV_SELFTEST, 0);
    CHECK(!stub_ir_loop, "A + B leaves the factory test for the bring-up test");
    CHECK(!app_stage(&st), "without passing it");

    /* no echo: after 10 s it asks, and keeps listening */
    for (int k = 0; k < 3; k++) {
        log_clear();
        to_ir();
        factory_frames(FPS + 5 * FPS, false);
        tap(B);
        CHECK(!logged("PASS"), "B does nothing while the IR step still waits");
        factory_frames(5 * FPS, false);
        CHECK(logged("ir: nothing heard yet") && !logged("FAIL") && stub_ir_loop,
              "10 s without an echo: it asks, still listening [%s]", stub_log);
        if (k == 0) {
            tap(B);
            factory_frames(1, false);
            CHECK(logged("passed without hearing") && logged("factory test: PASS") && app_stage(&st) &&
                      st == STAGE_TESTED,
                  "B passes it anyway, and says so [%s]", stub_log);
        } else if (k == 1) {
            factory_frames(FPS + 1, true);
            CHECK(logged("ir: test packet heard") && logged("factory test: PASS") && !logged("passed without"),
                  "an echo after the question still passes on its own [%s]", stub_log);
        } else {
            tap(A);
            CHECK(logged("FAIL at IR: nothing heard") && !stub_ir_loop, "A fails it [%s]", stub_log);
            tap(B);
            CHECK(logged("5/5 IR") && stub_ir_loop, "and B tries the IR step again");
        }
        factory_frames(3 * FPS + 2, false);
    }
}

static void usb(void)
{
    uint8_t out[PROTO_PACKET + 8], in[PROTO_PACKET + 8];
    init_name("ME");
    int n = proto_handle((const uint8_t *)"\x01", 1, out);
    CHECK(n == 6 && out[0] == 1 && out[1] == PROTO_VERSION && out[2] == app_items() && out[3] == 5 && out[4] == NAME_MAX &&
              out[5] == 'c',
          "INFO");
    static const uint8_t set_name[] = {3, 5, 'h', 'e', 'l', 'l', 'o'};
    n = proto_handle(set_name, sizeof set_name, out);
    CHECK(n == 7 && out[1] == 5 && !memcmp(out + 2, "HELLO", 5), "SET NAME replies with the cleaned-up name");
    name_t nm;
    CHECK(app_saved_name(&nm) && strcmp(nm.s, "HELLO") == 0, "SET NAME saves");
    static const uint8_t short_name[] = {3, 20, 'A'};
    n = proto_handle(short_name, sizeof short_name, out);
    CHECK(n == 2 && out[0] == 0xFF && out[1] == 3, "SET NAME longer than the packet is refused");
    static const uint8_t long_name[] = {3, NAME_MAX + 1};
    CHECK(proto_handle(long_name, 2, out) == 2 && out[0] == 0xFF, "SET NAME over NAME_MAX is refused");
    in[0] = 5;
    in[1] = (uint8_t)item_of("EDIT NAME");
    CHECK(proto_handle(in, 2, out) == 2 && out[0] == 0xFF, "SET ITEM EDIT NAME is refused");
    in[1] = 4;
    n = proto_handle(in, 2, out);
    CHECK(n == 4 && out[0] == 4 && out[1] == 4 && app_item() == 4, "SET ITEM 4 replies with the state");
    in[0] = 6;
    in[1] = 5;
    CHECK(proto_handle(in, 2, out) == 2 && out[0] == 0xFF, "SET LEVEL 5 is refused");
    in[0] = 7;
    in[1] = (uint8_t)(app_items() - 1);
    n = proto_handle(in, 2, out);
    CHECK(n == 3 + (int)strlen(app_label(in[1])) && out[2] == strlen(app_label(in[1])) &&
              !memcmp(out + 3, app_label(in[1]), out[2]),
          "GET LABEL of the last item");
    in[1] = app_items();
    CHECK(proto_handle(in, 2, out) == 2 && out[0] == 0xFF, "GET LABEL past the end is refused");
    CHECK(proto_handle(in, 0, out) == 2 && out[0] == 0xFF && out[1] == 0, "an empty packet is refused");
    CHECK(proto_handle((const uint8_t *)"\x42", 1, out) == 2 && out[0] == 0xFF && out[1] == 0x42, "unknown commands are refused");
    in[0] = 5;
    CHECK(proto_handle(in, 1, out) == 2 && out[0] == 0xFF, "SET ITEM without its argument is refused");

    /* fuzz: every reply has a sane length, nothing reads or writes out of bounds (ASan) */
    srand(42);
    for (int i = 0; i < 200000; i++) {
        int len = rand() % (PROTO_PACKET + 1);
        for (int k = 0; k < PROTO_PACKET; k++)
            in[k] = (uint8_t)rand();
        if (i % 3 == 0)
            in[0] = (uint8_t)(1 + rand() % 7);
        if (i % 7 == 0 && len > 1)
            in[1] = (uint8_t)(rand() % 30);
        uint8_t *exact = malloc((size_t)(len ? len : 1)); /* ASan catches reads past len */
        memcpy(exact, in, (size_t)len);
        n = proto_handle(exact, len, out);
        free(exact);
        if (n < 1 || n > PROTO_PACKET) {
            CHECK(0, "reply length %d for command %d, length %d", n, in[0], len);
            break;
        }
        if (i % 1000 == 0)
            app_render(0);
    }
}

/* a scrolling label moves one column a frame, without jumps */
static void label(void)
{
    static uint8_t g[2][GRID_H][GRID_W];
    const char *s = "HACKER HOTEL";
    int period = text_width(s) + 8, jumps = 0;
    for (int t = 0; t < 2 * period; t++) {
        for (int k = 0; k < 2; k++) {
            memset(fb, 0, sizeof fb);
            draw_label(s, 4, (uint32_t)(t + k), 10, 255);
            for (int r = 0; r < GRID_H; r++)
                for (int c = 0; c < GRID_W; c++)
                    g[k][r][c] = GRID[r][c] == SLOT_NONE ? 0 : fb[GRID[r][c]];
        }
        for (int r = 0; r < GRID_H; r++)
            for (int c = 0; c < GRID_W - 1; c++)
                if (GRID[r][c] != SLOT_NONE && GRID[r][c + 1] != SLOT_NONE && g[1][r][c] != g[0][r][c + 1])
                    jumps++;
    }
    CHECK(jumps == 0, "label scroll: %d jumps over two periods", jumps);
}

/* ◀ / ▶ outside the menu step through the screens, skipping EDIT NAME and
 * LIGHT (docs/USAGE.md "Name tag, Hacker Hotel and the animations") */
static void browse(void)
{
    init_name("ME");
    tap(R);
    CHECK(app_item() == 1, "▶ from NAME: HACKER HOTEL");
    tap(L);
    tap(L);
    CHECK(app_item() == item_of("VICTORY"), "◀ from NAME skips LIGHT and EDIT NAME to VICTORY (got %d)", app_item());
    /* is_game() once counted VICTORY as a game: this tap started an
     * invisible game instead */
    tap(R);
    CHECK(app_item() == 0, "▶ from VICTORY wraps to NAME (got %d)", app_item());
}

int main(void)
{
    browse();
    names();
    editor();
    infrared();
    console();
    factory();
    usb();
    label();
    TEST_END("app");
}
