/* Reboot into the CH643's factory boot ROM, which speaks WCH's ISP protocol
 * over USB. The badge has no debug pads, so after the first flash this is
 * the way to update it.
 *
 * Same as WCH's SystemReset_StartMode(Start_Mode_BOOT) followed by
 * NVIC_SystemReset(). Not tested on hardware. */
#include "ch643.h"
#include "bootloader.h"

#define FLASH_KEY1 0x45670123u
#define FLASH_KEY2 0xCDEF89ABu

void reboot_to_boot_rom(void)
{
    FLASH->KEYR = FLASH_KEY1; /* unlock the flash controller */
    FLASH->KEYR = FLASH_KEY2;
    FLASH->BOOT_MODEKEYR = FLASH_KEY1; /* unlock the boot-mode switch */
    FLASH->BOOT_MODEKEYR = FLASH_KEY2;
    FLASH->STATR |= FLASH_STATR_BOOT_MODE;
    FLASH->CTLR |= FLASH_CTLR_LOCK;
    NVIC_SystemReset();
    for (;;)
        ;
}
