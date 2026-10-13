/* Hacker Hotel 2027 starfish badge: C port of the theoretical Rust/Embassy
 * firmware, for comparison. Same hardware notes as ../../src/main.rs.
 *
 * One super-loop: SysTick runs free on HCLK/8 and the loop polls it to
 * schedule button scans (100 Hz), the CHRG pin (2 Hz) and frames (FPS).
 * USB and the UART are polled too. Interrupts: the LED scanner at the end
 * of each row (ledpwm.c), and infrared (ir.c): the SysTick compare while
 * sending, and the receiver's edges. */
#include <stdbool.h>
#include "ch643.h"
#include "anim.h"
#include "app.h"
#include "buttons.h"
#include "challenge.h"
#include "events.h"
#include "ir.h"
#include "log.h"
#include "ledpwm.h"
#include "selftest.h"
#include "store.h"
#include "uart.h"
#include "usb.h"

#define CHRG_PIN 22 /* PB22, XT4054 CHRG: open-drain, low while charging */

/* PC3 sees VBUS (through R33 to GND): high while USB powers the badge */
bool board_vbus(void)
{
    return GPIOC->INDR & (1u << 3);
}

static inline uint32_t ticks(void)
{
    return (uint32_t)SysTick->CNT;
}

static inline bool due(uint32_t *next, uint32_t period)
{
    if ((int32_t)(ticks() - *next) < 0)
        return false;
    *next += period;
    return true;
}

int main(void)
{
    SystemCoreClockUpdate();
    const uint32_t tick_ms = SystemCoreClock / 8000;
    SysTick->CTLR = (1u << 5) | (1u << 0); /* reset count, enable, HCLK/8, count up */

    RCC->APB2PCENR |= RCC_IOPAEN | RCC_IOPBEN;
    /* PB22 input with pull-up: CFGXR nibble 6 = 0b1000, pull-up via BSXR */
    GPIOB->CFGXR = (GPIOB->CFGXR & ~(0xFu << 24)) | (0x8u << 24);
    GPIOB->BSXR = 1u << (CHRG_PIN - 16);

    ledpwm_init();
    buttons_init();
    name_t name;
    if (!store_load(&name))
        name_default(&name);
    app_init(&name);
    challenge_load(store_progress());
    stage_t stage = store_stage();
    usb_init();
    uart_init();
    ir_init();
    log_str("\r\nhh27 starfish badge, c firmware. Name: ");
    log_line(name.s);
    log_line("Type help.");
    if (stage == STAGE_NEW)
        app_factory();
    else if (stage == STAGE_TESTED)
        app_sponsors();

    uint32_t next_buttons = ticks(), next_chrg = ticks(), next_frame = ticks();
    int charging = -1;
    for (;;) {
        usb_poll();
        uart_poll();
        ir_poll();
        if (due(&next_buttons, BUTTONS_POLL_MS * tick_ms))
            buttons_poll();
        if (due(&next_chrg, 500 * tick_ms)) {
            int c = !(GPIOB->INDR & (1u << CHRG_PIN));
            if (c != charging)
                app_event(EV_CHARGING, (uint8_t)(charging = c));
        }
        if (due(&next_frame, 1000 * tick_ms / FPS)) {
            app_render(buttons_held);
            ledpwm_show(app_brightness());
        }
        if (app_saved_name(&name)) {
            store_save(&name); /* a page erase and program: a few ms */
            log_str("name saved: ");
            log_line(name.s);
        }
        if (challenge_take_changed()) {
            store_set_progress(challenge_packed());
            log_str("challenges: ");
            log_u32((uint32_t)challenge_total());
            log_line(" of 25");
        }
        if (app_stage(&stage) && stage > store_stage())
            store_set_stage(stage); /* only forward: a re-run test does not bring the reel back */
    }
}
