/* IR send and receive, with two small interrupts and no timer peripheral.
 *
 * Send: the SysTick compare interrupt (the counter keeps running free for
 * the main loop) switches the LED for each half of the 38 kHz carrier
 * during a mark (1/3 on, 2/3 off), and fires once at the end of a space.
 * That is about 76 000 interrupts a second while a mark goes out, and
 * none otherwise. Doing it in software avoids the timer pin remaps, which
 * WCH has not documented for the CH643 (the KiCad symbol and ch32-data
 * disagree about TIM1 CH4 on PB23).
 *
 * Receive: an EXTI interrupt on both edges of PB19 stores how long each
 * mark and space lasted in a ring; ir_poll() decodes them in the main
 * loop. The receiver hears our own LED, so receiving pauses while we send.
 *
 * GUESS: SysTick compare without auto-reload. On QingKe V4 the CNTIF flag
 * should set when CNT reaches CMP with STRE = 0, as ch32fun relies on for
 * the V2 core; not checked on a CH643. */
#include "ch643.h"
#include "app.h"
#include "ir.h"
#include "irproto.h"

#define LED_ON_PIN 21  /* PB21, LED cathode: low = on (while the anode is high) */
#define ANODE_PIN 23   /* PB23, LED anode */
#define RECV_PIN 19    /* PB19, receiver output */
#define CARRIER_HZ 38000
#define ECHO_US 3000   /* the receiver's output settles after our last mark */

#ifndef IRQ
#define IRQ __attribute__((interrupt)) /* (empty for the host test) */
#endif

#define STK_STE (1u << 0)
#define STK_STIE (1u << 1)

static uint32_t ticks_per_us; /* SysTick runs on HCLK/8 */
static uint32_t on_ticks, off_ticks;

static struct {
    volatile bool busy;
    uint8_t buf[IR_PACKET_MAX];
    uint16_t seg, segs; /* segment being sent, and how many there are */
    uint16_t cycles;    /* carrier cycles left in this mark */
    bool lit;
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

static inline void led(bool on)
{
    if (on)
        GPIOB->BCR = 1u << LED_ON_PIN;
    else
        GPIOB->BSXR = 1u << (LED_ON_PIN - 16);
    tx.lit = on;
}

void ir_init(void)
{
    ticks_per_us = SystemCoreClock / 8000000;
    uint32_t period = SystemCoreClock / 8 / CARRIER_HZ;
    on_ticks = period / 3;
    off_ticks = period - on_ticks;

    RCC->APB2PCENR |= RCC_IOPBEN | RCC_AFIOEN;
    /* PB21, PB23 push-pull outputs (mode nibble 0b0011), both low: LED off
     * and no current; PB19 floating input (R35 pulls it up) */
    GPIOB->BCR = 1u << LED_ON_PIN | 1u << ANODE_PIN;
    GPIOB->CFGXR = (GPIOB->CFGXR & ~0xF0F0F000u) | 0x30304000u;

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
        /* done */
        SysTick->CTLR = STK_STE;
        GPIOB->BCR = 1u << ANODE_PIN;
        rx.mute_until = (uint32_t)SysTick->CNT + ECHO_US * ticks_per_us;
        rx.muted = true;
        tx.busy = false;
        return;
    }
    if (mark) {
        tx.cycles = (uint16_t)((us * CARRIER_HZ + 500000) / 1000000);
        led(true);
        tx.cmp += on_ticks;
    } else {
        tx.cycles = 0;
        tx.cmp += us * ticks_per_us;
    }
    cmp_set(tx.cmp);
}

IRQ void SysTick_Handler(void)
{
    SysTick->SR = 0;
    if (tx.lit) {
        led(false);
        tx.cmp += off_ticks;
        cmp_set(tx.cmp);
        --tx.cycles;
    } else if (tx.cycles) {
        led(true);
        tx.cmp += on_ticks;
        cmp_set(tx.cmp);
    } else {
        next_segment();
    }
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
    GPIOB->BSXR = 1u << (LED_ON_PIN - 16) | 1u << (ANODE_PIN - 16); /* cathode high first: off */
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
