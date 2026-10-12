/* Normally pulls in WCH's peripheral library headers. This port only uses
 * the register definitions in ch643.h, so it just declares the one library
 * function WCH's system_ch643.c calls (implemented in quirks.c). */
#pragma once

void GPIO_IPD_Unused(void);
