/* The UART pins of the debug header J3: a debug log out (log.h) and a small text console in
 * (console.c). 115200 baud, 8N1. Polled from the main loop. */
#pragma once

void uart_init(void);
/* send what is queued, read what came in; call it every loop */
void uart_poll(void);
