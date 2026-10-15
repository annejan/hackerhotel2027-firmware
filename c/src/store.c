/* The name in flash, same as ../../src/store.rs. NOT TESTED ON HARDWARE.
 *
 * The last two 256-byte pages of the code flash (0x0800F600 and 0x0800F700)
 * take turns. Each holds a magic word, a length and the name, in word 60 a
 * sequence number, in word 61 a check over the rest, in word 62 the
 * challenges' progress and in its last word the stage (store.h). Saving one
 * keeps the others. A save always writes the page that is *not* current, so
 * losing power halfway (a flat battery, a pulled cable) leaves a page that
 * fails its check, and the previous one still loads. The newest page that
 * passes its check is the current one.
 *
 * The Makefile links with a copy of WCH's Link.ld whose code flash ends two
 * pages early, so the firmware can never be placed there. `wchisp flash`
 * only erases the sectors the new firmware needs (plus one), so the name
 * and the stage survive an update as long as the image stays under 60 KiB
 * (CI checks that); `wchisp erase` clears them.
 *
 * Erase and program follow WCH's FLASH_ROM_ERASE / FLASH_ROM_WRITE
 * (ch643_flash.c, 256-byte "fast" pages): unlock with the flash and the
 * fast-mode keys, erase the page, fill the page buffer word by word,
 * program, lock. */
#include "ch643.h"
#include "store.h"

#define PAGE_A 0x0800F600u
#define PAGE_B 0x0800F700u
#define MAGIC 0x4E414D45u /* "NAME" */
#define SEQ_WORD 60
#define CHECK_WORD 61
#define PROGRESS_WORD 62
#define PROGRESS 0x50520000u /* "PR" in the top half, the progress below */
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
#define SR_WRPRTERR (1u << 4)
#define SR_EOP (1u << 5)

static inline uint32_t at(uint32_t page, int i)
{
    return *(volatile uint32_t *)(uintptr_t)(page + 4u * (uint32_t)i);
}

/* over every word but the check itself; an erased page does not pass */
static uint32_t check(const volatile uint32_t *w)
{
    uint32_t c = 0x48483237u; /* "HH27" */
    for (int i = 0; i < 64; i++)
        if (i != CHECK_WORD)
            c = (c << 5 | c >> 27) ^ w[i];
    return c;
}

static bool valid(uint32_t page)
{
    return at(page, 0) == MAGIC && at(page, CHECK_WORD) == check((const volatile uint32_t *)(uintptr_t)page);
}

/* the newest page that passes its check, or 0 when neither does */
static uint32_t current(void)
{
    bool a = valid(PAGE_A), b = valid(PAGE_B);
    if (a && b)
        return (int32_t)(at(PAGE_B, SEQ_WORD) - at(PAGE_A, SEQ_WORD)) > 0 ? PAGE_B : PAGE_A;
    return a ? PAGE_A : b ? PAGE_B : 0;
}

/* word i of the current page; erased (all ones) when there is none */
static uint32_t word(int i)
{
    uint32_t page = current();
    return page ? at(page, i) : 0xFFFFFFFFu;
}

bool store_load(name_t *out)
{
    uint32_t page = current();
    uint32_t len = page ? at(page, 1) : 0;
    if (len == 0 || len > NAME_MAX)
        return false;
    char buf[NAME_MAX];
    for (uint32_t i = 0; i < len; i++)
        buf[i] = (char)(at(page, 2 + (int)(i / 4)) >> (8 * (i % 4)));
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

/* Write the page that is not current, numbered one past it. false when the
 * controller reports an error or the page does not read back; the current
 * page then stays what it was, and the next save tries again. */
static bool program(uint32_t *page)
{
    uint32_t cur = current();
    /* a page is good for ~100k erases: never spend one on no change */
    if (cur) {
        bool same = true;
        for (int i = 0; i < 64; i++)
            same &= i == SEQ_WORD || i == CHECK_WORD || at(cur, i) == page[i];
        if (same)
            return true;
    }
    uint32_t to = cur == PAGE_A ? PAGE_B : PAGE_A;
    page[0] = MAGIC;
    page[SEQ_WORD] = cur ? at(cur, SEQ_WORD) + 1 : 1;
    page[CHECK_WORD] = check(page);

    FLASH->KEYR = KEY1;
    FLASH->KEYR = KEY2;
    FLASH->MODEKEYR = KEY1; /* fast (page) mode */
    FLASH->MODEKEYR = KEY2;

    ctlr(CR_PAGE_ER, CR_OPTER | CR_PAGE_ER);
    FLASH->ADDR = to;
    ctlr(CR_STRT, 0);
    wait();
    ctlr(0, CR_PAGE_ER);

    ctlr(CR_PAGE_PG, CR_OPTER | CR_PAGE_ER);
    ctlr(CR_BUF_RST, 0);
    wait();
    for (int i = 0; i < 64; i++) {
        *(volatile uint32_t *)(uintptr_t)(to + 4u * (uint32_t)i) = page[i];
        ctlr(CR_BUF_LOAD, 0);
        wait();
    }
    FLASH->ADDR = to;
    ctlr(CR_STRT, 0);
    wait();
    ctlr(0, CR_PAGE_PG);

    ctlr(CR_FLOCK | CR_LOCK, 0);
    bool ok = !(FLASH->STATR & SR_WRPRTERR);
    FLASH->STATR = SR_WRPRTERR | SR_EOP; /* write 1 to clear */
    for (int i = 0; i < 64; i++)
        ok &= at(to, i) == page[i];
    return ok;
}

bool store_save(const name_t *name)
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
    page[PROGRESS_WORD] = word(PROGRESS_WORD);
    page[STAGE_WORD] = word(STAGE_WORD);
    return program(page);
}

static bool set_word(int i, uint32_t v)
{
    uint32_t page[64], cur = current();
    for (int k = 0; k < 64; k++)
        page[k] = cur ? at(cur, k) : 0xFFFFFFFFu;
    if (!cur)
        page[1] = 0; /* no name yet */
    page[i] = v;
    return program(page);
}

uint16_t store_progress(void)
{
    uint32_t w = word(PROGRESS_WORD);
    return (w & 0xFFFF0000u) == PROGRESS ? (uint16_t)w : 0;
}

bool store_set_progress(uint16_t packed)
{
    return set_word(PROGRESS_WORD, PROGRESS | packed);
}

stage_t store_stage(void)
{
    uint32_t w = word(STAGE_WORD);
    return w == DONE ? STAGE_DONE : w == TESTED ? STAGE_TESTED : STAGE_NEW;
}

bool store_set_stage(stage_t stage)
{
    return set_word(STAGE_WORD, stage == STAGE_DONE ? DONE : stage == STAGE_TESTED ? TESTED : 0xFFFFFFFFu);
}
