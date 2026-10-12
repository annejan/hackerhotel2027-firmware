/* Capacitive touch pads T1..T4 on PC0..PC3 (ADC/TouchKey channels 10..13).
 *
 * Sequence from WCH's EVT example (EXAM/TOUCHKEY/TKey): enable TKEY once,
 * then per measurement select the channel, write the charge time to
 * IDATAR1 and the discharge time to RDATAR (starts it), wait for EOC and
 * read RDATAR. Thresholds are untuned.
 *
 * Datasheet: ADC channels 3/7/11/15 are missing on some silicon lots, so
 * T2 (channel 11) may never fire on those chips. */
#include <stdbool.h>
#include "ch643.h"
#include "events.h"
#include "touch.h"

#define PADS 4
#define FIRST_CHANNEL 10
#define HOLD_TICKS (TOUCH_HOLD_MS / TOUCH_POLL_MS)
#define BOOT_TICKS (TOUCH_BOOT_MS / TOUCH_POLL_MS)
#define CHARGE_TIME 0x80
#define DISCHARGE_TIME 0x08
#define SAMPLE_11_CYCLES 7
#define PRESS_DELTA 80
#define RELEASE_DELTA 40

static struct {
    uint32_t baseline; /* untouched level, 4 fractional bits */
    bool down;
    bool released; /* released during the last poll */
    bool chord;    /* was down together with another pad: no Tap/Hold of its own */
    uint16_t ticks;
} pads[PADS];

static uint32_t measure(int ch)
{
    ADC1->RSQR3 = (uint32_t)ch; /* SQ1 = channel, sequence length 1 */
    ADC1->IDATAR1 = CHARGE_TIME;
    ADC1->RDATAR = DISCHARGE_TIME;
    while (!(ADC1->STATR & ADC_EOC))
        ;
    return ADC1->RDATAR & 0x0FFF;
}

static inline uint32_t absdiff(uint32_t a, uint32_t b)
{
    return a > b ? a - b : b - a;
}

void touch_init(void)
{
    RCC->APB2PCENR |= RCC_IOPCEN | RCC_ADC1EN;
    GPIOC->CFGLR &= ~0xFFFFu; /* PC0..PC3 analog input */

    ADC1->CTLR3 = (ADC1->CTLR3 & ~0xFFu) | 0x25; /* ADC_CLK_Div6, as the WCH example */
    ADC1->CTLR1 = 0;                             /* independent, no scan */
    ADC1->CTLR2 = ADC_EXTSEL;                    /* software trigger, single, right-aligned */
    ADC1->RSQR1 = 0;                             /* 1 conversion */
    for (int i = 0; i < PADS; i++)
        ADC1->SAMPTR1 |= SAMPLE_11_CYCLES << (3 * (FIRST_CHANNEL + i - 10));
    ADC1->CTLR2 |= ADC_ADON;
    ADC1->CTLR1 |= 1u << 24; /* TKEY enable */

    for (int i = 0; i < PADS; i++)
        pads[i].baseline = measure(FIRST_CHANNEL + i) << 4;
}

void touch_poll(void)
{
    int down = 0;
    for (int i = 0; i < PADS; i++) {
        uint32_t raw = measure(FIRST_CHANNEL + i);
        /* direction of change with a finger is undocumented: react to both */
        uint32_t delta = absdiff(raw, pads[i].baseline >> 4);

        pads[i].released = false;
        if (!pads[i].down) {
            if (delta > PRESS_DELTA) {
                pads[i].down = true;
                pads[i].ticks = 0;
            } else {
                /* follow slow drift while untouched */
                pads[i].baseline = pads[i].baseline - (pads[i].baseline >> 5) + ((raw << 4) >> 5);
            }
        } else if (delta < RELEASE_DELTA) {
            pads[i].down = false;
            pads[i].released = true;
        } else if (pads[i].ticks < UINT16_MAX) {
            pads[i].ticks++;
        }
        down += pads[i].down;
    }

    for (int i = 0; i < PADS; i++) {
        if (down > 1 && pads[i].down)
            pads[i].chord = true;
        if (pads[i].released) {
            if (!pads[i].chord && pads[i].ticks < HOLD_TICKS)
                anim_event(EV_TAP, (uint8_t)i);
            pads[i].chord = false;
        } else if (pads[i].down && !pads[i].chord && pads[i].ticks == HOLD_TICKS) {
            anim_event(EV_HOLD, (uint8_t)i);
        }
    }

    const uint16_t a = pads[TOUCH_BOOT_A].ticks, b = pads[TOUCH_BOOT_B].ticks;
    if (pads[TOUCH_BOOT_A].down && pads[TOUCH_BOOT_B].down && (a < b ? a : b) == BOOT_TICKS)
        anim_event(EV_BOOTLOADER, 0);
}
