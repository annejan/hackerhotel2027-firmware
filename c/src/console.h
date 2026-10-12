/* A small text console on the UART header (uart.c feeds it lines):
 *   help             list the commands
 *   info             firmware, name, what is showing, brightness
 *   name TEXT        set the name (saved to flash, like EDIT NAME)
 *   show N           show item N (as the USB SET ITEM)
 *   level N          brightness 0..4
 *   hello            send the name over IR, as B on the name tag does */
#pragma once

#define CONSOLE_LINE 48

void console_line(const char *line);
