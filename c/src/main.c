/* Hacker Hotel 2027 starfish badge: C port of the theoretical Rust/Embassy
 * firmware, for comparison. Same hardware notes as ../../src/main.rs.
 *
 * One super-loop, no interrupts: SysTick runs free on HCLK/8 and the loop
 * polls it to schedule touch scans (100 Hz), the CHRG pin (2 Hz) and
 * frames (FPS). */
#include <stdbool.h>
#include "ch643.h"
#include "anim.h"
#include "events.h"
#include "ledpwm.h"
#include "touch.h"

#define CHRG_PIN 22 /* PB22, XT4054 CHRG: open-drain, low while charging */

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
    touch_init();
    anim_init();

    uint32_t next_touch = ticks(), next_chrg = ticks(), next_frame = ticks();
    int charging = -1;
    for (;;) {
        if (due(&next_touch, TOUCH_POLL_MS * tick_ms))
            touch_poll();
        if (due(&next_chrg, 500 * tick_ms)) {
            int c = !(GPIOB->INDR & (1u << CHRG_PIN));
            if (c != charging)
                anim_event(EV_CHARGING, (uint8_t)(charging = c));
        }
        if (due(&next_frame, 1000 * tick_ms / FPS)) {
            anim_render();
            ledpwm_show(anim_brightness());
        }
    }
}
