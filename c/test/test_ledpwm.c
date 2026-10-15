/* ledpwm.c against a fake LEDPWM block: pin and block setup, the row
 * order and DMA words the interrupt loads, the frame swap at the start of
 * a scan, brightness to intensity, and that ledpwm_show() does not hang
 * when the scanner stops. Includes the driver to see its row buffers. */
#include "check.h"
#include "ledpwm.c"

/* what the hardware does at the end of a row, then the interrupt */
static void end_of_row(void)
{
    LEDPWM->PWM_OE = 0;
    LEDPWM->DMA_CNT = 0;
    LEDPWM_IRQHandler();
}

int main(void)
{
    ledpwm_init();
    CHECK(fake_gpioa.CFGLR == 0x33333333 && fake_gpioa.CFGHR == 0x33333333 && fake_gpioa.CFGXR == 0x33333333 &&
              fake_gpioa.BCR == 0xFFFFFF,
          "PA0..PA23 push-pull outputs, low");
    CHECK(fake_gpiob.CFGLR == 0x33333333 && fake_gpiob.CFGHR == 0x33333333 && fake_gpiob.BSHR == 0xFFFF &&
              fake_gpiob.CFGXR == 0,
          "PB0..PB15 push-pull outputs, high; PB16 and up untouched");
    CHECK(LEDPWM->COM_E_PB_L == 0xFF && LEDPWM->COM_E_PB_M == 0xFF && !LEDPWM->COM_E_PB_H && !LEDPWM->COM_E_PAPC,
          "COM lines PB0..PB15");
    CHECK(LEDPWM->PWM_PIN_L == 0xFF && LEDPWM->PWM_PIN_H == 0 && LEDPWM->PWM_MOD == 0x71, "PA24 matrix mode, 8 RGB columns");
    CHECK(LEDPWM->CYCLE_CFG == 0x20 && LEDPWM->FRAME_CFG == 0x30 && LEDPWM->FUNC_CTRL == 0x85, "timing and row interrupt");
    CHECK(fake_nvic[LEDPWM_IRQn], "LEDPWM interrupt enabled");
    CHECK(LEDPWM->COM_SEL == 0x10 && LEDPWM->DMA_CNT == 8 && LEDPWM->PWM_OE == 1, "row 0 started on PB0");

    for (int i = 0; i < SLOTS; i++)
        fb[i] = (uint8_t)i; /* each slot holds its own index */
    ledpwm_show(255);
    CHECK(pending, "the frame waits for the start of a scan");
    int order = 1, data = 1, dma = 1;
    for (int r = 1; r <= 2 * COMS; r++) {
        end_of_row();
        int c = r % COMS;
        if (LEDPWM->COM_SEL != 0x10 + c || LEDPWM->DMA_CNT != 8 || LEDPWM->PWM_OE != 1)
            order = 0;
        if (LEDPWM->DMA != (uint16_t)(uintptr_t)rows[front][c])
            dma = 0;
        if (r >= COMS) /* after the first wrap the new frame shows */
            for (int k = 0; k < 8; k++) {
                uint32_t w = rows[front][c][k];
                int s = c * PWM_CHANNELS;
                if ((w & 0xFF) != 63 || (w >> 8 & 0xFF) != GAMMA[(uint8_t)(s + k)] ||
                    (w >> 16 & 0xFF) != GAMMA[(uint8_t)(s + k + 8)] || (w >> 24) != GAMMA[(uint8_t)(s + k + 16)])
                    data = 0;
            }
    }
    CHECK(order, "COM order PB0..PB15, 8 words a row, PWM_OE on again");
    CHECK(dma, "DMA points at the row being loaded");
    CHECK(data, "row word = intensity | PA k | PA k+8 | PA k+16, through GAMMA");
    CHECK(!pending, "swapped at the start of the scan");
    CHECK(row_count == 2 * COMS, "row counter (%u)", (unsigned)row_count);

    static const int map[][2] = {{0, 0}, {1, 1}, {4, 1}, {12, 3}, {32, 8}, {80, 20}, {255, 63}};
    for (unsigned i = 0; i < sizeof map / sizeof map[0]; i++) {
        ledpwm_show((uint8_t)map[i][0]);
        for (int r = 0; r < COMS; r++)
            end_of_row();
        CHECK((rows[front][0][0] & 0xFF) == (uint32_t)map[i][1], "brightness %d -> intensity %d (got %u)", map[i][0],
              map[i][1], rows[front][0][0] & 0xFF);
    }

    ledpwm_show(32);
    ledpwm_show(32); /* no scan in between: gives up waiting */
    CHECK(pending, "a second show without a scan returns");
    TEST_END("ledpwm");
}
