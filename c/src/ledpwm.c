/* CH643 LEDPWM matrix driver, from the CH643 reference manual (CH643RM
 * V1.9, chapter 24). Same as ../../src/ledpwm.rs. NOT TESTED ON HARDWARE.
 *
 * The board is the manual's "8 columns RGB, built-in drive" case (24.2.2;
 * reference circuit 24.4.2 is 16 rows x 8 RGB columns, like this badge):
 * PA0..PA23 are the PWM channels (the chip sources current, active high,
 * through the series resistors to the anodes), PB0..PB15 the COM lines
 * (built-in NMOS sinks the cathodes, active low). In MATRIX_MODE PA24 the
 * PWM pins form 8 columns of three: column k is PA k ("red"), PA k+8
 * ("green") and PA k+16 ("blue"). Here they are simply 24 channels.
 *
 * The scanner does not run by itself: it shows one COM line, stops and
 * raises its interrupt. The handler points the DMA at the next row (8
 * words; per column the intensity byte, then the PA k, PA k+8, PA k+16
 * levels), writes DMA_CNT = 8, selects the next COM (which clears the
 * flag) and sets PWM_OE again (PWM_OE_CLR switched it off for the change,
 * so the rows do not ghost into each other).
 *
 * Timing: intensity PWM 6 bits (64 clocks), colour PWM 8 bits (256
 * intensity periods): 16384 clocks a row, 341 us at 48 MHz, 5.5 ms for
 * the 16 rows, 183 Hz. The brightness goes in the intensity byte, so the
 * badge dims without losing any of the 256 colour steps.
 *
 * GUESS (the manual does not say): the LED reference clock is HCLK; with
 * the 6-bit intensity PWM the intensity byte's low 6 bits count; DMA
 * addresses are the low 16 bits of a RAM address. */
#include <stdbool.h>
#include "ch643.h"
#include "frame.h"
#include "ledpwm.h"

#ifndef IRQ
#define IRQ __attribute__((interrupt)) /* (empty for the host test) */
#endif

/* register bits, CH643RM 24.3 */
#define COM_SEL_PB0 0x10       /* COM_SEL code of PB0; PBn is 0x10 + n */
#define MATRIX_PA24 0x01       /* PWM_MOD: PA[7:0] / PA[15:8] / PA[23:16] */
#define PWM_RGB_EN 0x70        /* PWM_MOD: red, green and blue groups on */
#define INTEN_6BIT 0x20        /* CYCLE_CFG: 6-bit intensity, 8-bit colour, clock / 1 */
#define INHIBIT(n) (((n) - 1) << 4) /* FRAME_CFG: blanking in intensity periods */
#define IE_INHIBIT 0x80        /* FUNC_CTRL: interrupt at the end of each row */
#define PWM_OE_CLR 0x04        /* FUNC_CTRL: clear PWM_OE on that interrupt */
#define LED_ENABLE 0x01        /* FUNC_CTRL: the LED clock */
#define ROW_WORDS 8            /* 8 columns, one 32-bit word each */

/* two frames of row data; the scanner reads `front`, ledpwm_show() fills
 * the other one and the handler swaps them at the start of a scan */
static uint32_t rows[2][COMS][ROW_WORDS] __attribute__((aligned(4)));
static volatile uint8_t front, com;
static volatile bool pending;
static volatile uint32_t row_count;

uint32_t ledpwm_rows(void)
{
    return row_count;
}

static void load_row(uint8_t c)
{
    LEDPWM->DMA = (uint16_t)(uintptr_t)rows[front][c];
    LEDPWM->DMA_CNT = ROW_WORDS;
    LEDPWM->COM_SEL = (uint8_t)(COM_SEL_PB0 + c); /* clears the interrupt flag */
    LEDPWM->PWM_OE = 1;
}

IRQ void LEDPWM_IRQHandler(void)
{
    row_count++;
    uint8_t c = (uint8_t)(com + 1);
    if (c == COMS) {
        c = 0;
        if (pending) {
            front ^= 1;
            pending = false;
        }
    }
    com = c;
    load_row(c);
}

void ledpwm_init(void)
{
    RCC->APB2PCENR |= RCC_IOPAEN | RCC_IOPBEN;
    /* PWM pins PA0..PA23: push-pull outputs, OUTDR 0 = active high.
     * COM pins PB0..PB15: push-pull outputs, OUTDR 1 = active low.
     * (RM 24.2.2 and table 8-10; with the LED block off this also keeps
     * every LED dark.) */
    GPIOA->BCR = 0x00FFFFFFu;
    GPIOA->CFGLR = 0x33333333u;
    GPIOA->CFGHR = 0x33333333u;
    GPIOA->CFGXR = 0x33333333u;
    GPIOB->BSHR = 0xFFFFu;
    GPIOB->CFGLR = 0x33333333u;
    GPIOB->CFGHR = 0x33333333u;

    LEDPWM->COM_E_PB_L = 0xFF; /* PB0..PB15 are COM lines */
    LEDPWM->COM_E_PB_M = 0xFF;
    LEDPWM->COM_E_PB_H = 0x00;
    LEDPWM->COM_E_PAPC = 0x00;
    LEDPWM->PWM_PIN_L = 0xFF; /* columns 0..7 */
    LEDPWM->PWM_PIN_H = 0x00;
    LEDPWM->PWM_MOD = PWM_RGB_EN | MATRIX_PA24;
    LEDPWM->ANA_CTRL = 0x00; /* no built-in constant current: the board has resistors */
    LEDPWM->CYCLE_CFG = INTEN_6BIT;
    LEDPWM->FRAME_CFG = INHIBIT(4); /* ~5 us between rows, no repeats */
    LEDPWM->KEY_CTRL = 0x00;
    LEDPWM->FUNC_CTRL = IE_INHIBIT | PWM_OE_CLR | LED_ENABLE;
    /* row 0 first, then the interrupt: loading the row clears a flag the
     * idle block may already have raised, so the handler cannot run before
     * it and start on row 1 while row 0 is being loaded */
    com = 0;
    load_row(0);
    NVIC_EnableIRQ(LEDPWM_IRQn);
}

void ledpwm_show(uint8_t brightness)
{
    /* the last frame is still waiting for the start of a scan (5.5 ms at
     * most); give up after a while instead of hanging if the scanner stopped,
     * and once it has stopped do not wait again: every 30 ms spent spinning
     * here would starve USB, the UART and the buttons */
    static bool stalled;
    for (uint32_t i = 0; pending && !stalled && i < 200000; i++)
        ;
    stalled = pending;
    uint8_t inten = brightness >> 2; /* 6-bit intensity */
    if (brightness && !inten)
        inten = 1;
    uint32_t(*back)[ROW_WORDS] = rows[front ^ 1];
    for (int c = 0; c < COMS; c++) {
        const uint8_t *px = fb + c * PWM_CHANNELS;
        for (int k = 0; k < ROW_WORDS; k++)
            back[c][k] = inten | (uint32_t)GAMMA[px[k]] << 8 | (uint32_t)GAMMA[px[k + 8]] << 16 |
                         (uint32_t)GAMMA[px[k + 16]] << 24;
    }
    pending = true;
}
