/* What the badge shows, and the controls; same as ../../src/app.rs.
 *
 * Anywhere:
 *   hold A            open the menu (also from a game)
 *   hold ◀ + ▶, 3 s   reboot into the USB boot ROM
 * Name, hotel and animations:
 *   ◀ / ▶             previous / next item
 *   A                 brightness
 * Menu:
 *   ◀ / ▶             previous / next item, A select, hold A close
 * EDIT NAME: see editor.c; the new name comes out of app_saved_name() for
 * main.c to keep in flash.
 * Games: demo mode until you press ◀, B or ▶; then the game owns the
 * buttons (see games/). */
#include <stdbool.h>
#include <string.h>
#include "anim.h"
#include "app.h"
#include "bootloader.h"
#include "editor.h"
#include "frame.h"
#include "game.h"
#include "nametag.h"

enum {
    NAME,
    HOTEL,
    RIPPLE,
    GALAXY,
    HEARTBEAT,
    LIFE,
    FIRE,
    TWINKLE,
    FIRST_GAME, /* the games, in game_kind_t order */
    EDIT_NAME = FIRST_GAME + GAMES,
    LIGHT,
    ITEMS
};

static const char *const LABELS[ITEMS] = {
    "NAME", "HACKER HOTEL", "RIPPLE", "GALAXY", "HEART", "LIFE", "FIRE", "TWINKLE",
    "SNAKE", "STARTRIS", "BREAKOUT", "PONG", "FLAPPY", "STAR CHASE", "EDIT NAME", "LIGHT",
};

/* brightness steps, as a share of full PWM duty (applied after gamma):
 * night, 5%, 12.5% (the default), 31%, full */
static const uint8_t LEVELS[5] = {4, 12, 32, 80, 255};
#define MENU_TIMEOUT (20 * FPS)

static struct {
    uint8_t item, cursor, level;
    bool menu, charging, show_charge;
    uint32_t idle, t;
    uint8_t boot_frames; /* > 0: showing the USB boot screen, then reboot */
    name_t name;
    bool editing;
    bool has_saved; /* `saved` waits for main.c to keep it in flash */
    name_t saved;
} a;

static bool is_game(int i)
{
    return i >= FIRST_GAME && i < EDIT_NAME;
}

static void select_item(int i)
{
    a.item = (uint8_t)(i % ITEMS);
    a.t = 0;
    if (a.item == LIFE)
        life_init();
    else if (a.item == FIRE)
        fire_reset();
    else if (is_game(a.item))
        game_start((game_kind_t)(a.item - FIRST_GAME));
    else if (a.item == EDIT_NAME) {
        editor_start(&a.name);
        a.editing = true;
    }
}

bool app_saved_name(name_t *out)
{
    if (!a.has_saved)
        return false;
    *out = a.saved;
    a.has_saved = false;
    return true;
}

static void editor_done(edit_t done, const name_t *name)
{
    if (done == EDIT_SAVE) {
        a.name = *name;
        a.saved = *name;
        a.has_saved = true;
    }
    a.editing = false;
    select_item(NAME);
}

static int step(int i, int d)
{
    return (i + d + ITEMS) % ITEMS;
}

/* quick browse outside the menu: EDIT NAME and LIGHT are only menu
 * entries, skip them */
static void browse(int d)
{
    int i = step(a.item, d);
    while (i == EDIT_NAME || i == LIGHT)
        i = step(i, d);
    select_item(i);
}

void app_init(const name_t *name)
{
    memset(&a, 0, sizeof a);
    a.level = 2;
    a.name = *name;
    frame_init();
    life_init();
    fire_reset();
    game_start(GAME_SNAKE);
}

void app_event(event_t ev, uint8_t arg)
{
    if (ev == EV_BOOTLOADER) {
        a.boot_frames = FPS; /* show it for a second, then reboot */
        return;
    }
    a.idle = 0;
    if (ev == EV_CHARGING) {
        a.charging = arg;
        a.show_charge = arg && !(is_game(a.item) && game_playing()) && !a.menu && !a.editing;
        return;
    }
    if (a.menu) {
        if (ev == EV_TAP && arg == 0)
            a.cursor = (uint8_t)step(a.cursor, -1);
        else if (ev == EV_TAP && arg == 3)
            a.cursor = (uint8_t)step(a.cursor, 1);
        else if (ev == EV_TAP && arg == 2 && a.cursor == LIGHT)
            a.level = (uint8_t)((a.level + 1) % sizeof LEVELS);
        else if (ev == EV_TAP && arg == 2) {
            a.menu = false;
            select_item(a.cursor);
        } else if (ev == EV_HOLD && arg == 2)
            a.menu = false;
        return;
    }
    if (a.editing) {
        name_t name;
        edit_t done = editor_event(ev, arg, &name);
        if (done != EDIT_BUSY)
            editor_done(done, &name);
        return;
    }
    if (a.show_charge) {
        if (ev == EV_TAP)
            a.show_charge = false;
        return;
    }
    if (is_game(a.item) && game_event(ev, arg))
        return;
    if (ev == EV_HOLD && arg == 2) {
        a.menu = true;
        a.cursor = a.item;
    } else if (ev == EV_TAP && arg == 0) {
        browse(-1);
    } else if (ev == EV_TAP && arg == 3) {
        browse(1);
    } else if (ev == EV_TAP && arg == 2) {
        a.level = (uint8_t)((a.level + 1) % sizeof LEVELS);
    }
}

uint8_t app_brightness(void)
{
    uint8_t b = LEVELS[a.level];
    /* make sure the boot screen is visible even in night mode */
    return a.boot_frames && b < 32 ? 32 : b;
}

/* menu: the item's name on the grid, its place in the list on the outline,
 * and its group in the arms' colour */
static void draw_menu(uint32_t t)
{
    int c = a.cursor;
    draw_label(LABELS[c], 4, t, FPS, 255);
    int lo = c * EDGE_LEDS / ITEMS, hi = (c + 1) * EDGE_LEDS / ITEMS;
    for (int j = 0; j < EDGE_LEDS; j++)
        fb[EDGE[j]] = j >= lo && j < hi ? 220 : 12;
    rgb_t group = c == NAME || c == HOTEL ? (rgb_t){255, 150, 20}
                  : is_game(c)            ? (rgb_t){20, 255, 70}
                  : c >= EDIT_NAME        ? (rgb_t){255, 255, 255}
                                          : (rgb_t){170, 40, 255};
    for (int i = 0; i < RGB_LEDS; i++)
        fb_rgb(i, rgb_scaled(group, 110));
    if (c == LIGHT)
        for (int k = 0; k < (int)sizeof LEVELS; k++) {
            uint8_t v = k == a.level ? 255 : k < a.level ? 90 : 15;
            for (int x = 0; x < 2; x++)
                fb_grid(13, 3 + k * 2 + x, v);
        }
}

void app_render(uint8_t held)
{
    memset(fb, 0, sizeof fb);
    if (a.boot_frames) {
        if (--a.boot_frames == 0)
            reboot_to_boot_rom();
        anim_usb_boot(FPS - 1 - a.boot_frames);
        return;
    }
    uint32_t t = a.t++;
    if (a.menu) {
        if (++a.idle > MENU_TIMEOUT)
            a.menu = false;
        draw_menu(t);
        return;
    }
    if (a.editing) {
        edit_t done = editor_render(held);
        if (done != EDIT_BUSY)
            editor_done(done, &a.name);
        return;
    }
    if (a.show_charge) {
        anim_charging(t);
        return;
    }
    switch (a.item) {
    case NAME: nametag_name(t, &a.name); break;
    case HOTEL: nametag_hotel(t); break;
    case RIPPLE: anim_ripple(t); break;
    case GALAXY: anim_galaxy(t); break;
    case HEARTBEAT: anim_heartbeat(t); break;
    case LIFE: life_render(t); break;
    case FIRE: fire_render(); break;
    case TWINKLE: anim_twinkle(t); break;
    case EDIT_NAME:
    case LIGHT: select_item(NAME); break; /* menu entries only */
    default: game_render(held); break;
    }
}

uint8_t app_items(void) { return ITEMS; }
const char *app_label(uint8_t i) { return i < ITEMS ? LABELS[i] : ""; }
uint8_t app_item(void) { return a.item; }
uint8_t app_levels(void) { return sizeof LEVELS; }
uint8_t app_level(void) { return a.level; }
bool app_charging(void) { return a.charging; }
void app_name(name_t *out) { *out = a.name; }

bool app_set_item(uint8_t i)
{
    if (i >= ITEMS || i == EDIT_NAME || i == LIGHT)
        return false;
    a.menu = a.editing = a.show_charge = false;
    select_item(i);
    return true;
}

void app_set_level(uint8_t level)
{
    if (level < sizeof LEVELS)
        a.level = level;
}

void app_set_name(const name_t *name)
{
    a.name = *name;
    a.saved = *name;
    a.has_saved = true;
    if (a.editing) { /* the editor would hold an old copy */
        a.editing = false;
        select_item(NAME);
    }
}
