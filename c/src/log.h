/* A debug log: the UART header on the badge (uart.c), stderr in the
 * emulator (../emu/emu.c). Never blocks; text that does not fit the
 * buffer is dropped. */
#pragma once
#include <stdint.h>

void log_str(const char *s);
void log_u32(uint32_t n);
/* s, then a line end */
void log_line(const char *s);
