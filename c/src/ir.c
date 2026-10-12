/* IR send and receive.
 *
 * Send: TIM1 channel 4 makes the 38 kHz carrier (1/3 duty) on PB23, the
 * LED's anode; TIM1_RM = 1xx puts CH4 there (CH643RM 8.3.2.1). The
 * cathode PB21 stays low. The SysTick compare interrupt (the counter keeps
 * running free for the main loop) fires at the end of each mark and space
 * and switches the carrier on or off for the next one: about 450
 * interrupts for a name. The compare value goes through the timer's
 * preload, so every edge is late by the same part of a carrier period.
 *
 * Receive: an EXTI interrupt on both edges of PB19 stores how long each
 * mark and space lasted in a ring; ir_poll() decodes them in the main
 * loop. The receiver hears our own LED, so receiving pauses while we send.
 *
 * SysTick with STRE = 0 keeps counting past the compare value and sets
 * CNTIF when it gets there (QingKe V4 manual, chapter 5). */
#include "ch643.h"
#include "app.h"
#include "ir.h"
#include "irproto.h"

#define CATHODE_PIN 21 /* PB21, LED cathode: always low */
#define ANODE_PIN 23   /* PB23, LED anode: TIM1 CH4 */
#define RECV_PIN 19    /* PB19, receiver output */
#define CARRIER_HZ 38000
#define ECHO_US 3000   /* the receiver's output settles after our last mark */

#ifndef IRQ
#define IRQ __attribute__((interrupt)) /* (empty for the host test) */
#endif

#define STK_STE (1u << 0)
#define STK_STIE (1u << 1)

static uint32_t ticks_per_us; /* SysTick runs on HCLK/8 */
static uint16_t carrier_on;   /* TIM1 compare value for 1/3 duty */

static struct {
    volatile bool busy;
    uint8_t buf[IR_PACKET_MAX];
    uint16_t seg, segs; /* segment being sent, and how many there are */
    uint64_t cmp;
} tx;

static struct {
    volatile uint16_t ring[64]; /* bit 15 = mark, then microseconds */
    volatile uint8_t head, tail;
    uint32_t last;    /* SysTick at the last edge */
    volatile bool muted;
    uint32_t mute_until; /* SysTick: our own echo is over */
    ir_dec_t dec;
} rx;

/* write the high word first: the compare value only moves forward, so the
 * half-written value is in the future and cannot fire early */
static void cmp_set(uint64_t v)
{
    volatile uint32_t *c = (volatile uint32_t *)&SysTick->CMP;
    c[1] = (uint32_t)(v >> 32);
    c[0] = (uint32_t)v;
}

void ir_init(void)
{
    ticks_per_us = SystemCoreClock / 8000000;
    uint16_t period = (uint16_t)(SystemCoreClock / CARRIER_HZ);
    carrier_on = period / 3;

    RCC->APB2PCENR |= RCC_IOPBEN | RCC_AFIOEN | RCC_TIM1EN;
    /* TIM1 CH4 on PB23 (TIM1_RM = 100), PWM mode 1 with preload, output
     * low while the compare value is 0; the counter only runs while sending */
    AFIO->PCFR1 = (AFIO->PCFR1 & ~AFIO_PCFR1_TIM1_REMAP) | AFIO_PCFR1_TIM1_REMAP_2;
    TIM1->PSC = 0;
    TIM1->ATRLR = period - 1;
    TIM1->CHCTLR2 = (uint16_t)((TIM1->CHCTLR2 & 0x00FF) | 0x6000 | TIM_OC4PE);
    TIM1->CH4CVR = 0;
    TIM1->CCER |= TIM_CC4E;
    TIM1->BDTR |= TIM_MOE;
    TIM1->SWEVGR = TIM_UG;
    /* PB21 push-pull output low, PB23 alternate push-pull (0b1011), PB19
     * floating input (R35 pulls it up) */
    GPIOB->BCR = 1u << CATHODE_PIN;
    GPIOB->CFGXR = (GPIOB->CFGXR & ~0xF0F0F000u) | 0xB0304000u;

    /* EXTI line 19 from port B, both edges */
    AFIO->EXTICR[1] = (AFIO->EXTICR[1] & ~(3u << 6)) | 1u << 6;
    EXTI->RTENR |= 1u << RECV_PIN;
    EXTI->FTENR |= 1u << RECV_PIN;
    EXTI->INTFR = 1u << RECV_PIN;
    EXTI->INTENR |= 1u << RECV_PIN;
    NVIC_EnableIRQ(EXTI25_16_IRQn);
    NVIC_EnableIRQ(SysTicK_IRQn);
    rx.last = (uint32_t)SysTick->CNT;
}

bool ir_busy(void)
{
    return tx.busy;
}

/* segments: 0 header mark, 1 header space, then a mark and a space per
 * bit, then the end mark */
static void next_segment(void)
{
    uint16_t s = tx.seg++;
    uint32_t us;
    bool mark;
    if (s == 0) {
        mark = true, us = IR_HDR_MARK_US;
    } else if (s == 1) {
        mark = false, us = IR_HDR_SPACE_US;
    } else if (s < tx.segs) {
        uint16_t k = (uint16_t)((s - 2) / 2);
        mark = (s & 1) == 0 || s == tx.segs - 1;
        us = mark ? IR_BIT_MARK_US : tx.buf[k / 8] >> (k % 8) & 1 ? IR_ONE_SPACE_US : IR_ZERO_SPACE_US;
    } else {
        /* done: carrier off, timer stopped */
        SysTick->CTLR = STK_STE;
        TIM1->CH4CVR = 0;
        TIM1->SWEVGR = TIM_UG;
        TIM1->CTLR1 = 0;
        rx.mute_until = (uint32_t)SysTick->CNT + ECHO_US * ticks_per_us;
        rx.muted = true;
        tx.busy = false;
        return;
    }
    TIM1->CH4CVR = mark ? carrier_on : 0;
    tx.cmp += us * ticks_per_us;
    cmp_set(tx.cmp);
}

IRQ void SysTick_Handler(void)
{
    SysTick->SR = 0;
    next_segment();
}

bool ir_send(uint8_t type, const uint8_t *data, int len)
{
    if (tx.busy)
        return false;
    int n = ir_pack(type, data, len, tx.buf);
    if (!n)
        return false;
    tx.seg = 0;
    tx.segs = (uint16_t)(2 + n * 8 * 2 + 1);
    tx.busy = true;
    TIM1->CNT = 0;
    TIM1->CTLR1 = TIM_ARPE | TIM_CEN;
    tx.cmp = SysTick->CNT + 20 * ticks_per_us;
    cmp_set(tx.cmp);
    SysTick->SR = 0;
    SysTick->CTLR = STK_STE | STK_STIE;
    return true;
}

IRQ void EXTI25_16_IRQHandler(void)
{
    uint32_t now = (uint32_t)SysTick->CNT;
    EXTI->INTFR = 1u << RECV_PIN;
    bool low = !(GPIOB->INDR & (1u << RECV_PIN));
    uint32_t us = (now - rx.last) / ticks_per_us;
    rx.last = now;
    if (tx.busy || rx.muted)
        return;
    /* the interval that just ended: a space if the output is low now */
    uint16_t v = (uint16_t)(us > 0x7FFF ? 0x7FFF : us) | (low ? 0 : 0x8000);
    uint8_t next = (uint8_t)((rx.head + 1) % 64);
    if (next != rx.tail) {
        rx.ring[rx.head] = v;
        rx.head = next;
    }
}

void ir_poll(void)
{
    if (rx.muted && (int32_t)((uint32_t)SysTick->CNT - rx.mute_until) >= 0) {
        rx.muted = false;
        ir_dec_reset(&rx.dec);
    }
    while (rx.tail != rx.head) {
        uint16_t v = rx.ring[rx.tail];
        rx.tail = (uint8_t)((rx.tail + 1) % 64);
        int n = ir_dec_feed(&rx.dec, v & 0x8000, v & 0x7FFF);
        if (n)
            app_ir_received(rx.dec.buf[0], rx.dec.buf + 2, rx.dec.buf[1]);
    }
}
