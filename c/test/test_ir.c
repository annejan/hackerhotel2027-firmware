/* ir.c and irproto.c against a fake SysTick, TIM2, EXTI and GPIO: the pin
 * and timer setup, the carrier waveform that ir_send() produces (decoded
 * back), received edges with the receiver's distortion, the echo mute,
 * ring overflow, and a decoder fuzz. Interrupts are called by hand. */
#include <stdlib.h>
#include <string.h>
#include "ch643.h"
#include "check.h"
#include "ir.h"
#include "irproto.h"

void SysTick_Handler(void);
void EXTI25_16_IRQHandler(void);

static uint8_t got[64];
static int got_len, got_type, ngot;
void app_ir_received(uint8_t type, const uint8_t *d, int len)
{
    got_type = type;
    got_len = len;
    memcpy(got, d, (size_t)len);
    ngot++;
}

#define TICKS_PER_US 6 /* SysTick on HCLK / 8 = 6 MHz */

/* run a transmission to its end, calling the SysTick compare interrupt
 * when it is due; returns the carrier envelope as marks and spaces */
static int envelope(int *mark, uint32_t *us, int max, uint32_t *on_t, uint32_t *period)
{
    int n = 0, cur = 0;
    uint64_t since = fake_systick.CNT;
    *on_t = 0;
    *period = fake_tim2.ATRLR + 1u;
    while (ir_busy()) {
        if (!(fake_systick.CTLR & 2)) { CHECK(0, "SysTick compare interrupt off while busy"); break; }
        if (!(fake_tim2.CTLR1 & 1)) { CHECK(0, "TIM2 stopped while busy"); break; }
        fake_systick.CNT = fake_systick.CMP;
        SysTick_Handler();
        int on = fake_tim2.CH1CVR != 0;
        if (on)
            *on_t = fake_tim2.CH1CVR;
        if (on != cur) {
            if (fake_systick.CNT != since && n < max) {
                mark[n] = cur;
                us[n++] = (uint32_t)((fake_systick.CNT - since) / TICKS_PER_US);
            }
            cur = on;
            since = fake_systick.CNT;
        }
    }
    if (cur && n < max) {
        mark[n] = 1;
        us[n++] = 0;
    }
    return n;
}

/* the receiver output on PB19 (low = mark) changes level after us */
static void edge(int level, uint32_t us)
{
    fake_systick.CNT += (uint64_t)us * TICKS_PER_US;
    if (level)
        fake_gpiob.INDR |= 1u << 19;
    else
        fake_gpiob.INDR &= ~(1u << 19);
    EXTI25_16_IRQHandler();
}

/* a whole packet on the receiver pin; marks stretched by `stretch` us, as
 * the receiver does; ir_poll() every `poll_every` bits (0: never) */
static void receive(const uint8_t *pkt, int plen, int gap, int stretch, int poll_every)
{
    edge(0, (uint32_t)gap);
    edge(1, (uint32_t)(IR_HDR_MARK_US + stretch));
    edge(0, (uint32_t)(IR_HDR_SPACE_US - stretch));
    for (int k = 0; k < plen * 8; k++) {
        edge(1, (uint32_t)(IR_BIT_MARK_US + stretch));
        edge(0, (uint32_t)((pkt[k / 8] >> (k % 8) & 1 ? IR_ONE_SPACE_US : IR_ZERO_SPACE_US) - stretch));
        if (poll_every && k % poll_every == 0)
            ir_poll();
    }
    edge(1, (uint32_t)(IR_BIT_MARK_US + stretch));
}

static uint32_t seed = 1;
static uint32_t rnd(void) { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; return seed; }

int main(void)
{
    fake_gpiob.OUTDR = 0;
    ir_init();
    /* BSXR / BCR as the hardware applies them */
    fake_gpiob.OUTDR |= (fake_gpiob.BSXR & 0xFF) << 16;
    CHECK((fake_gpiob.CFGXR >> 12 & 0xF) == 4 && (fake_gpiob.CFGXR >> 20 & 0xF) == 0xB && (fake_gpiob.CFGXR >> 28 & 0xF) == 8,
          "PB19 floating in, PB21 alternate push-pull, PB23 pull-up (CFGXR %08x)", fake_gpiob.CFGXR);
    CHECK((fake_gpiob.OUTDR >> 21 & 1) && (fake_gpiob.OUTDR >> 23 & 1), "PB21 idles high (LED off), PB23 pulled up");
    CHECK((fake_afio.PCFR1 >> 18 & 7) == 1, "TIM2_RM = 001 (CH1 on PB21)");
    CHECK(fake_rcc.APB1PCENR & 1, "TIM2 clock on");
    CHECK((fake_tim2.CHCTLR1 >> 4 & 7) == 6 && (fake_tim2.CHCTLR1 & 8) && (fake_tim2.CCER & 0xF) == 3,
          "TIM2 CH1 PWM mode 1, preload, enabled, inverted");
    CHECK(fake_tim2.BDTR & TIM_MOE, "TIM2 main output enabled (an advanced timer on the CH643)");
    CHECK((fake_afio.EXTICR[1] >> 6 & 3) == 1, "EXTI19 on port B");
    CHECK(fake_nvic[EXTI25_16_IRQn], "EXTI25_16 interrupt enabled");

    /* send: the waveform decodes back to the packet */
    const char *name = "HACKER HOTEL FAN";
    fake_systick.CNT = 0xFFFFFFF0ull; /* low word about to wrap */
    CHECK(ir_send(IR_HELLO, (const uint8_t *)name, (int)strlen(name)), "send");
    CHECK(!ir_send(IR_HELLO, (const uint8_t *)name, 3), "a second send while busy is refused");
    static int mark[1000];
    static uint32_t us[1000];
    uint32_t on_t, period;
    uint64_t t0 = fake_systick.CNT;
    int n = envelope(mark, us, 1000, &on_t, &period);
    uint64_t air_us = (fake_systick.CNT - t0) / TICKS_PER_US;
    double khz = 48e3 / period;
    CHECK(khz > 37.6 && khz < 38.4, "carrier %.2f kHz", khz);
    CHECK(3 * on_t > period - 30 && 3 * on_t < period + 30, "carrier duty 1/3 (%u of %u)", on_t, period);
    CHECK(air_us > 200000 && air_us < 260000, "16-character name on air for %llu us", (unsigned long long)air_us);
    CHECK(fake_tim2.CH1CVR == 0 && !(fake_tim2.CTLR1 & 1), "carrier off (PB21 high), TIM2 stopped after");
    CHECK(!(fake_systick.CTLR & 2), "SysTick compare interrupt off after");
    ir_dec_t d;
    ir_dec_reset(&d);
    int r = 0;
    for (int i = 0; i < n && !r; i++)
        r = ir_dec_feed(&d, mark[i], us[i]);
    CHECK(r == (int)strlen(name) + 3 && d.buf[0] == IR_HELLO && !memcmp(d.buf + 2, name, strlen(name)),
          "the sent waveform decodes (r = %d)", r);

    /* receive, with the receiver's distortion (marks long, spaces short) */
    uint8_t pkt[IR_PACKET_MAX];
    int plen = ir_pack(IR_REPLY, (const uint8_t *)"ANNE JAN", 8, pkt);
    fake_gpiob.INDR = 1u << 19;
    fake_systick.CNT += 60000 * TICKS_PER_US; /* the echo mute is over */
    ir_poll();
    for (int stretch = 0; stretch <= 240; stretch += 60) {
        ngot = 0;
        receive(pkt, plen, 5000, stretch, 16);
        ir_poll();
        CHECK(ngot == 1 && got_type == IR_REPLY && got_len == 8 && !memcmp(got, "ANNE JAN", 8),
              "received with marks %d us long (got %d)", stretch, ngot);
    }

    /* our own echo is ignored while sending, and just after */
    CHECK(ir_send(IR_HELLO, (const uint8_t *)"X", 1), "send 2");
    ngot = 0;
    receive(pkt, plen, 100, 0, 0);
    envelope(mark, us, 1000, &on_t, &period);
    ir_poll();
    CHECK(ngot == 0, "a packet heard while sending is ignored");
    edge(0, 100); edge(1, 9000); edge(0, 4500); /* starts in the mute */
    ir_poll();
    fake_systick.CNT += 5000 * TICKS_PER_US;
    ir_poll();
    for (int k = 0; k < plen * 8; k++) {
        edge(1, 560);
        edge(0, pkt[k / 8] >> (k % 8) & 1 ? 1690 : 560);
        if (k % 16 == 0)
            ir_poll();
    }
    edge(1, 560);
    ir_poll();
    CHECK(ngot == 0, "a packet that started during the mute is not taken");
    receive(pkt, plen, 20000, 0, 16);
    ir_poll();
    CHECK(ngot == 1, "the next packet after the mute is received");

    /* no ir_poll() for a whole packet: the edge ring overflows, the packet
     * is lost, nothing breaks, and the next one is received again */
    ngot = 0;
    receive(pkt, plen, 20000, 0, 0);
    ir_poll();
    CHECK(ngot == 0, "an unpolled packet overflows the ring and is dropped (got %d)", ngot);
    receive(pkt, plen, 20000, 0, 16);
    ir_poll();
    CHECK(ngot == 1, "and the next polled packet is received (got %d)", ngot);

    /* a flash write stalls the CPU for ~5 ms mid-send: the next deadline
     * must not be left in the past, or the send never ends */
    CHECK(ir_send(IR_HELLO, (const uint8_t *)"LATE", 4), "send 3");
    fake_systick.CNT = fake_systick.CMP;
    SysTick_Handler(); /* header mark on */
    fake_systick.CNT = fake_systick.CMP + 6000 * TICKS_PER_US;
    SysTick_Handler();
    CHECK(fake_systick.CMP > fake_systick.CNT, "a deadline missed during a stall is moved ahead");
    envelope(mark, us, 1000, &on_t, &period);
    CHECK(!ir_busy() && fake_tim2.CH1CVR == 0, "and the send still ends, carrier off");

    /* decoder: random intervals never make a packet; truncated and
     * corrupted packets are refused */
    ir_dec_t fz;
    ir_dec_reset(&fz);
    int fake = 0;
    for (long i = 0; i < 2000000; i++)
        fake += ir_dec_feed(&fz, rnd() & 1, rnd() % 12000) != 0;
    CHECK(fake == 0, "2M random intervals: %d packets accepted", fake);
    CHECK(ir_pack(IR_HELLO, pkt, IR_DATA_MAX + 1, pkt) == 0, "ir_pack refuses more than IR_DATA_MAX");
    for (int bit = 0; bit < plen * 8; bit++) {
        uint8_t bad[IR_PACKET_MAX];
        memcpy(bad, pkt, (size_t)plen);
        bad[bit / 8] ^= (uint8_t)(1u << (bit % 8));
        ir_dec_reset(&fz);
        r = ir_dec_feed(&fz, true, IR_HDR_MARK_US);
        r |= ir_dec_feed(&fz, false, IR_HDR_SPACE_US);
        for (int k = 0; k < plen * 8 && !r; k++) {
            r |= ir_dec_feed(&fz, true, IR_BIT_MARK_US);
            r |= ir_dec_feed(&fz, false, bad[k / 8] >> (k % 8) & 1 ? IR_ONE_SPACE_US : IR_ZERO_SPACE_US);
        }
        if (!r)
            r = ir_dec_feed(&fz, true, IR_BIT_MARK_US);
        /* a flipped length bit may still end in a good CRC only by luck */
        CHECK(r == 0 || bit / 8 == 1, "a flipped bit %d is refused (r = %d)", bit, r);
    }
    TEST_END("ir");
}
