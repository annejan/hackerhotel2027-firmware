/* Register-level replacement for WCH's GPIO_IPD_Unused(), which their
 * SystemInit() calls: pads that exist on the die but are not bonded out in
 * the package must be pulled up, or they float and draw current.
 * Only the CH643Q (LQFP64) case: PC6..PC9, PC12, PC13 and PC20.
 * This board also leaves PC0..PC2, PC14 and PC15 unconnected (the CC
 * jumpers went in hardware commit 0986add), so those get a pull-up too.
 * PC3 (VBUS sense, 100k to GND) must stay a floating input. */
#include "ch643.h"

void GPIO_IPD_Unused(void)
{
    RCC->APB2PCENR |= RCC_IOPCEN;
    /* input with pull-up = mode nibble 0b1000, pull selected by OUTDR = 1 */
    GPIOC->CFGLR = (GPIOC->CFGLR & 0x00FFF000u) | 0x88000888u; /* PC0..PC2, PC6, PC7 */
    /* CFGHR is not readable on early silicon (WCH keeps a shadow copy), so
     * write it whole: PC8, PC9, PC12..PC15 pull-up, the rest floating input */
    GPIOC->CFGHR = 0x88884488u;
    GPIOC->BSHR = 0x7u | (1u << 6) | (1u << 7) | (1u << 8) | (1u << 9) | (0xFu << 12);
    GPIOC->CFGXR = (GPIOC->CFGXR & ~(0xFu << 16)) | (0x8u << 16); /* PC20 */
    GPIOC->BSXR = 1u << 4;
}
