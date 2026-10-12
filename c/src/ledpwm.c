/* CH643 LEDPWM matrix driver. THEORETICAL, NOT TESTED ON HARDWARE.
 *
 * Same register sequence as the Rust firmware's ledpwm.rs; see there for
 * what is known. In short: register names come from WCH's ch643.h, but
 * there is no public reference manual, so every value marked GUESS is
 * inferred from a field name and must be checked on real hardware. */
#include "ch643.h"
#include "frame.h"
#include "ledpwm.h"

/* GUESS: COM-major, PWM_CHANNELS grey bytes per COM, read by the LEDPWM DMA */
static uint8_t dma_buf[SLOTS];

void ledpwm_init(void)
{
    /* GUESS: no LEDPWM clock-enable bit exists in ch643.h, assume always clocked */

    /* COM lines: PB0..PB15. No COMs on port A or C: COM_PA[23:16]_EN must
     * stay 0, PA16..PA23 are PWM channels on this board. */
    LEDPWM->COM_E_PB_L = 0xFF;
    LEDPWM->COM_E_PB_M = 0xFF;
    LEDPWM->COM_E_PB_H = 0x00;
    LEDPWM->COM_E_PAPC = 0x00;

    /* GUESS: PWM_PIN has 16 bits; PA16..PA23 presumably via MATRIX_MODE */
    LEDPWM->PWM_PIN_L = 0xFF;
    LEDPWM->PWM_PIN_H = 0xFF;
    LEDPWM->PWM_MOD = 0x01;

    /* GUESS: COM count - 1, COM active low (built-in NMOS sinks cathodes) */
    LEDPWM->COM_SEL = COMS - 1;
    LEDPWM->KEY_CTRL = LED_KEY_CTRL_COM_LVE;

    /* GUESS: fastest PWM clock, 8-bit grey, no repeat/inhibit */
    LEDPWM->CYCLE_CFG = 0x00;
    LEDPWM->FRAME_CFG = 0x00;
    LEDPWM->ANA_CTRL = 0x00;
    LEDPWM->INTEN_ADJ = 0xFF;

    /* GUESS: bytes per COM - 1, low 16 bits of the RAM address */
    LEDPWM->DMA_CNT = PWM_CHANNELS - 1;
    LEDPWM->DMA = (uint16_t)(uintptr_t)dma_buf;

    LEDPWM->PWM_OE = LED_PWM_OE_PWM_OE;
    LEDPWM->FUNC_CTRL = LED_FUNC_CTRL_LED_ENABLE;
}

void ledpwm_show(uint8_t brightness)
{
    /* GUESS: LED_IF_SET marks the end of a scan frame (write 1 to clear);
     * give up after a while instead of hanging if that is wrong */
    for (int i = 0; i < 2000; i++) {
        if (LEDPWM->DISP_STAT & LED_DISP_STAT_LED_IF_SET) {
            LEDPWM->DISP_STAT = LED_DISP_STAT_LED_IF_SET;
            break;
        }
    }
    for (int i = 0; i < SLOTS; i++)
        dma_buf[i] = GAMMA[scale8(fb[i], brightness)];
}
