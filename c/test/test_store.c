/* store.c: the flash page with the name, the challenges' progress and the
 * first-boot stage. The page is mapped at its real address (0x0800F700)
 * in this process, so store.c runs unchanged; the flash controller is a
 * fake that only records what was written to it (the page writes land in
 * the mapping directly). Checks the layout, that saving one thing keeps
 * the others, that bad pages are refused, and the unlock / lock sequence.
 * Linux only (MAP_FIXED_NOREPLACE). */
#define _GNU_SOURCE
#include <string.h>
#include <sys/mman.h>
#include "ch643.h"
#include "check.h"
#include "store.h"

#define PAGE 0x0800F700u
static volatile uint32_t *page;

static void erase(void) { memset((void *)page, 0xFF, 256); }

static void lock_check(const char *after)
{
    CHECK((fake_flash.CTLR & (1u << 7)) && (fake_flash.CTLR & (1u << 15)), "%s: flash locked again", after);
    CHECK(!(fake_flash.CTLR & (3u << 16)), "%s: page program / erase bits cleared", after);
    CHECK(fake_flash.KEYR == 0xCDEF89ABu && fake_flash.MODEKEYR == 0xCDEF89ABu && fake_flash.ADDR == PAGE,
          "%s: unlock keys and page address", after);
    fake_flash.CTLR = fake_flash.KEYR = fake_flash.MODEKEYR = fake_flash.ADDR = 0;
}

int main(void)
{
    void *m = mmap((void *)(uintptr_t)(PAGE & ~0xFFFu), 4096, PROT_READ | PROT_WRITE,
                   MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (m == MAP_FAILED) {
        perror("mmap at the flash page");
        return 1;
    }
    page = (volatile uint32_t *)(uintptr_t)PAGE;

    name_t n, got;
    erase();
    CHECK(!store_load(&got), "erased page: no name");
    CHECK(store_stage() == STAGE_NEW, "erased page: a new badge");
    CHECK(store_progress() == 0, "erased page: no progress");

    name_set(&n, "ANNE JAN", 8);
    store_save(&n);
    lock_check("save");
    CHECK(page[0] == 0x4E414D45u && page[1] == 8, "magic \"NAME\" and length (%08x %u)", page[0], page[1]);
    CHECK(page[2] == ('A' | 'N' << 8 | 'N' << 16 | (uint32_t)'E' << 24), "characters packed little-endian (%08x)", page[2]);
    CHECK(store_load(&got) && strcmp(got.s, "ANNE JAN") == 0 && got.len == 8, "name loads back");
    store_save(&n);
    CHECK(fake_flash.KEYR == 0 && fake_flash.CTLR == 0, "saving the same name again does not erase the page");

    store_set_progress(0x0b6d);
    lock_check("progress");
    store_set_stage(STAGE_TESTED);
    CHECK(page[62] == 0x50520b6du && page[63] == 0x54455354u, "progress in word 62, stage in word 63 (%08x %08x)",
          page[62], page[63]);
    CHECK(store_progress() == 0x0b6d && store_stage() == STAGE_TESTED, "progress and stage read back");
    CHECK(store_load(&got) && strcmp(got.s, "ANNE JAN") == 0, "saving progress and stage keeps the name");

    name_set(&n, "ABCDEFGHIJKLMNOPQRSTUVWX", 24);
    store_save(&n);
    CHECK(store_load(&got) && got.len == NAME_MAX && strcmp(got.s, "ABCDEFGHIJKLMNOPQRSTUVWX") == 0,
          "a 24-character name loads back");
    CHECK(store_progress() == 0x0b6d && store_stage() == STAGE_TESTED, "saving the name keeps progress and stage");
    store_set_stage(STAGE_DONE);
    CHECK(store_stage() == STAGE_DONE && store_progress() == 0x0b6d, "stage DONE");
    store_set_stage(STAGE_NEW);
    CHECK(store_stage() == STAGE_NEW && page[63] == 0xFFFFFFFFu, "stage NEW is the erased word");

    page[1] = NAME_MAX + 1;
    CHECK(!store_load(&got), "length over NAME_MAX refused");
    page[1] = 0;
    CHECK(!store_load(&got), "length 0 refused");
    page[1] = 3;
    page[0] = 0x4E414D44u;
    CHECK(!store_load(&got), "wrong magic refused");
    page[0] = 0x4E414D45u;
    page[2] = 'a' | 0x01u << 8 | (uint32_t)'~' << 16;
    CHECK(store_load(&got) && strcmp(got.s, "A") == 0, "junk characters cleaned up like any name (\"%s\")", got.s);
    page[62] = 0x12340b6du;
    CHECK(store_progress() == 0, "progress without the \"PR\" tag is ignored");
    page[63] = 0x54455355u;
    CHECK(store_stage() == STAGE_NEW, "an unknown stage word is a new badge");
    TEST_END("store");
}
