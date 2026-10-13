/* The name in flash, same as ../../src/store.rs. NOT TESTED ON HARDWARE.
 *
 * The last 256-byte page of the code flash (0x0800F700) holds a magic word,
 * a length and the name, and in its last word the stage (store.h). Saving
 * one keeps the other. The Makefile links with a copy of WCH's Link.ld
 * whose code flash ends one page early, so the firmware can never be placed
 * there. `wchisp flash` only erases the sectors the new firmware needs (plus
 * one), so the name and the stage survive an update; `wchisp erase` clears
 * them.
 *
 * Erase and program follow WCH's FLASH_ROM_ERASE / FLASH_ROM_WRITE
 * (ch643_flash.c, 256-byte "fast" pages): unlock with the flash and the
 * fast-mode keys, erase the page, fill the page buffer word by word,
 * program, lock. */
#include "ch643.h"
#include "store.h"

#define PAGE 0x0800F700u
#define MAGIC 0x4E414D45u /* "NAME" */
#define STAGE_WORD 63
#define TESTED 0x54455354u /* "TEST" */
#define DONE 0x444F4E45u   /* "DONE"; anything else, erased too, is new */

#define KEY1 0x45670123u
#define KEY2 0xCDEF89ABu
/* FLASH_CTLR bits, from ch643_flash.c */
#define CR_OPTER (1u << 5)
#define CR_STRT (1u << 6)
#define CR_LOCK (1u << 7)
#define CR_FLOCK (1u << 15)
#define CR_PAGE_PG (1u << 16)
#define CR_PAGE_ER (1u << 17)
#define CR_BUF_LOAD (1u << 18)
#define CR_BUF_RST (1u << 19)
#define SR_BSY (1u << 0)

static inline uint32_t word(int i)
{
    return *(volatile uint32_t *)(PAGE + 4u * (uint32_t)i);
}

bool store_load(name_t *out)
{
    uint32_t len = word(1);
    if (word(0) != MAGIC || len == 0 || len > NAME_MAX)
        return false;
    char buf[NAME_MAX];
    for (uint32_t i = 0; i < len; i++)
        buf[i] = (char)(word(2 + (int)(i / 4)) >> (8 * (i % 4)));
    name_set(out, buf, (int)len);
    return true;
}

static void wait(void)
{
    while (FLASH->STATR & SR_BSY)
        ;
}

static void ctlr(uint32_t set, uint32_t clear)
{
    FLASH->CTLR = (FLASH->CTLR & ~clear) | set;
}

static void program(const uint32_t *page)
{
    FLASH->KEYR = KEY1;
    FLASH->KEYR = KEY2;
    FLASH->MODEKEYR = KEY1; /* fast (page) mode */
    FLASH->MODEKEYR = KEY2;

    ctlr(CR_PAGE_ER, CR_OPTER | CR_PAGE_ER);
    FLASH->ADDR = PAGE;
    ctlr(CR_STRT, 0);
    wait();
    ctlr(0, CR_PAGE_ER);

    ctlr(CR_PAGE_PG, CR_OPTER | CR_PAGE_ER);
    ctlr(CR_BUF_RST, 0);
    wait();
    for (int i = 0; i < 64; i++) {
        *(volatile uint32_t *)(PAGE + 4u * (uint32_t)i) = page[i];
        ctlr(CR_BUF_LOAD, 0);
        wait();
    }
    FLASH->ADDR = PAGE;
    ctlr(CR_STRT, 0);
    wait();
    ctlr(0, CR_PAGE_PG);

    ctlr(CR_FLOCK | CR_LOCK, 0);
}

void store_save(const name_t *name)
{
    uint32_t page[64];
    for (int i = 0; i < 64; i++)
        page[i] = 0xFFFFFFFFu;
    page[0] = MAGIC;
    page[1] = name->len;
    for (int i = 0; i < name->len; i++) {
        uint32_t *w = &page[2 + i / 4];
        *w = (*w & ~(0xFFu << (8 * (i % 4)))) | (uint32_t)(uint8_t)name->s[i] << (8 * (i % 4));
    }
    page[STAGE_WORD] = word(STAGE_WORD);
    program(page);
}

stage_t store_stage(void)
{
    uint32_t w = word(STAGE_WORD);
    return w == DONE ? STAGE_DONE : w == TESTED ? STAGE_TESTED : STAGE_NEW;
}

void store_set_stage(stage_t stage)
{
    uint32_t page[64];
    for (int i = 0; i < 64; i++)
        page[i] = word(i);
    page[STAGE_WORD] = stage == STAGE_DONE ? DONE : stage == STAGE_TESTED ? TESTED : 0xFFFFFFFFu;
    program(page);
}
