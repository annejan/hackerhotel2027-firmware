/* USART3 on the debug header J3: pin 2 RX (PB17), pin 4 TX (PB16), pin 5
 * VCC, pin 7 GND. Output goes through a ring buffer that uart_poll()
 * drains, so logging never waits for the line. Input comes in on the RX
 * interrupt into a second ring buffer: a byte takes 87 us and the USART
 * holds only one, so polling from the main loop would drop pasted text.
 *
 * USART3_RM = 11 puts TX3 / RX3 on PB16 / PB17 (CH643RM 8.3.2.1; CTS, CK
 * and RTS would land on PB18, PB22, PB19, but flow control and the clock
 * output stay off). */
#include "ch643.h"
#include "console.h"
#include "log.h"
#include "uart.h"

#define BAUD 115200
#define TXQ 512 /* power of two */
#define RXQ 128 /* power of two; a whole console line */
#define IRQ __attribute__((interrupt))

static struct {
    char q[TXQ];
    uint16_t head, tail;
    char line[CONSOLE_LINE];
    uint8_t n;
} u;

static struct {
    char q[RXQ];
    volatile uint8_t head; /* written by the interrupt only */
    uint8_t tail;
} rx;

IRQ void USART3_IRQHandler(void)
{
    while (USART3->STATR & (USART_STATR_RXNE | USART_STATR_ORE)) {
        char c = (char)USART3->DATAR; /* reading DATAR also clears an overrun */
        uint8_t next = (uint8_t)((rx.head + 1) & (RXQ - 1));
        if (next != rx.tail) { /* full: drop it */
            rx.q[rx.head] = c;
            rx.head = next;
        }
    }
}

void uart_init(void)
{
    RCC->APB2PCENR |= RCC_IOPBEN | RCC_AFIOEN;
    RCC->APB1PCENR |= RCC_USART3EN;
    AFIO->PCFR1 = (AFIO->PCFR1 & ~AFIO_PCFR1_USART3_REMAP) | AFIO_PCFR1_USART3_REMAP; /* = 3 */
    /* PB16 alternate push-pull 50 MHz (0b1011), PB17 floating input (R34
     * pulls it up) */
    GPIOB->CFGXR = (GPIOB->CFGXR & ~0xFFu) | 0x4Bu;
    USART3->BRR = (uint16_t)((SystemCoreClock + BAUD / 2) / BAUD); /* USART3 runs on HCLK */
    USART3->CTLR1 = USART_CTLR1_UE | USART_CTLR1_TE | USART_CTLR1_RE | USART_CTLR1_RXNEIE;
    NVIC_EnableIRQ(USART3_IRQn);
}

void uart_poll(void)
{
    while (u.tail != u.head && (USART3->STATR & USART_STATR_TXE)) {
        USART3->DATAR = (uint8_t)u.q[u.tail];
        u.tail = (u.tail + 1) & (TXQ - 1);
    }
    while (rx.tail != rx.head) {
        char c = rx.q[rx.tail];
        rx.tail = (uint8_t)((rx.tail + 1) & (RXQ - 1));
        if (c == '\r' || c == '\n') {
            if (u.n) {
                u.line[u.n] = 0;
                log_str("\r\n");
                console_line(u.line);
                u.n = 0;
            }
        } else if (c == '\b' || c == 0x7F) {
            if (u.n) {
                u.n--;
                log_str("\b \b");
            }
        } else if (c >= ' ' && u.n < CONSOLE_LINE - 1) {
            u.line[u.n++] = c;
            char echo[2] = {c, 0};
            log_str(echo);
        }
    }
}

void log_str(const char *s)
{
    while (*s) {
        uint16_t next = (u.head + 1) & (TXQ - 1);
        if (next == u.tail)
            return; /* full: drop the rest */
        u.q[u.head] = *s++;
        u.head = next;
    }
}

void log_u32(uint32_t n)
{
    char b[11];
    int i = 10;
    b[i] = 0;
    do {
        b[--i] = (char)('0' + n % 10);
        n /= 10;
    } while (n);
    log_str(b + i);
}

void log_line(const char *s)
{
    log_str(s);
    log_str("\r\n");
}
