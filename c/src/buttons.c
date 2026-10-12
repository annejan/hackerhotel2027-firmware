/* The four push buttons (see buttons.h): polled, debounced, turned into
 * press / tap / hold / chord events. Same rules as ../../src/buttons.rs. */
#include <stdbool.h>
#include "ch643.h"
#include "buttons.h"
#include "events.h"

#define BUTTONS 4
#define DEBOUNCE 2 /* a change has to be seen on this many polls in a row */
#define HOLD_TICKS (BUTTONS_HOLD_MS / BUTTONS_POLL_MS)
#define BOOT_TICKS (BUTTONS_BOOT_MS / BUTTONS_POLL_MS)

static GPIO_TypeDef *const PORT[BUTTONS] = {GPIOC, GPIOB, GPIOC, GPIOB};
static const uint8_t PIN[BUTTONS] = {5, 20, 4, 18};

static struct {
    bool down;
    uint8_t bounce; /* polls in a row that disagreed with `down` */
    bool released;  /* released during the last poll */
    bool chord;     /* was down together with another button: no Tap/Hold of its own */
    uint16_t ticks;
} buttons[BUTTONS];

volatile uint8_t buttons_held;

void buttons_init(void)
{
    RCC->APB2PCENR |= RCC_IOPBEN | RCC_IOPCEN;
    /* floating input (mode nibble 0b0100, the reset state; set anyway) */
    GPIOC->CFGLR = (GPIOC->CFGLR & ~0x00FF0000u) | 0x00440000u;  /* PC4, PC5 */
    GPIOB->CFGXR = (GPIOB->CFGXR & ~0x000F0F00u) | 0x00040400u;  /* PB18, PB20 */
}

void buttons_poll(void)
{
    int down = 0;
    for (int i = 0; i < BUTTONS; i++) {
        bool low = !(PORT[i]->INDR & (1u << PIN[i]));
        buttons[i].released = false;
        if (buttons[i].down && buttons[i].ticks < UINT16_MAX)
            buttons[i].ticks++;
        if (low == buttons[i].down) {
            buttons[i].bounce = 0;
        } else if (++buttons[i].bounce >= DEBOUNCE) {
            buttons[i].bounce = 0;
            buttons[i].down = low;
            if (low) {
                buttons[i].ticks = 0;
                app_event(EV_PRESS, (uint8_t)i);
            } else {
                buttons[i].released = true;
            }
        }
        down += buttons[i].down;
    }

    uint8_t held = 0;
    for (int i = 0; i < BUTTONS; i++)
        held |= (uint8_t)(buttons[i].down << i);
    buttons_held = held;

    for (int i = 0; i < BUTTONS; i++) {
        if (down > 1 && buttons[i].down)
            buttons[i].chord = true;
        if (buttons[i].released) {
            if (!buttons[i].chord && buttons[i].ticks < HOLD_TICKS)
                app_event(EV_TAP, (uint8_t)i);
            buttons[i].chord = false;
        } else if (buttons[i].down && !buttons[i].chord && buttons[i].ticks == HOLD_TICKS) {
            app_event(EV_HOLD, (uint8_t)i);
        }
    }

    const uint16_t a = buttons[BUTTONS_BOOT_A].ticks, b = buttons[BUTTONS_BOOT_B].ticks;
    if (buttons[BUTTONS_BOOT_A].down && buttons[BUTTONS_BOOT_B].down && (a < b ? a : b) == BOOT_TICKS)
        app_event(EV_BOOTLOADER, 0);
}
