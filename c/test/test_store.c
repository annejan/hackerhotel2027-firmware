/* store.c: the two flash pages with the name, the challenges' progress and
 * the first-boot stage. The pages are mapped at their real addresses
 * (0x0800F600 and 0x0800F700) in this process, so store.c runs unchanged;
 * the flash controller is a fake that only records what was written to it
 * (the page writes land in the mapping directly). Checks the layout, that
 * saves take turns between the pages, that a half-written page falls back
 * to the previous one, that saving one thing keeps the others, that bad
 * pages are refused, and the unlock / lock sequence.
 * Linux only (MAP_FIXED_NOREPLACE). */
#define _GNU_SOURCE
#include <string.h>
#include <sys/mman.h>
#include "ch643.h"
#include "check.h"
#include "store.h"

#define PAGE_A 0x0800F600u
#define PAGE_B 0x0800F700u
static volatile uint32_t *a, *b;

static void erase(volatile uint32_t *p) { memset((void *)p, 0xFF, 256); }

/* the check word, as store.c computes it: the format, written down twice */
static void reseal(volatile uint32_t *p)
{
    uint32_t c = 0x48483237u;
    for (int i = 0; i < 64; i++)
        if (i != 61)
            c = (c << 5 | c >> 27) ^ p[i];
    p[61] = c;
}

static void lock_check(const char *after, uint32_t page)
{
    CHECK((fake_flash.CTLR & (1u << 7)) && (fake_flash.CTLR & (1u << 15)), "%s: flash locked again", after);
    CHECK(!(fake_flash.CTLR & (3u << 16)), "%s: page program / erase bits cleared", after);
    CHECK(fake_flash.KEYR == 0xCDEF89ABu && fake_flash.MODEKEYR == 0xCDEF89ABu && fake_flash.ADDR == page,
          "%s: unlock keys and page address %08x (got %08x)", after, page, fake_flash.ADDR);
    CHECK(fake_flash.STATR == 0x30u, "%s: error and end-of-operation flags cleared", after);
    fake_flash.CTLR = fake_flash.KEYR = fake_flash.MODEKEYR = fake_flash.ADDR = fake_flash.STATR = 0;
}

int main(void)
{
    void *m = mmap((void *)(uintptr_t)(PAGE_A & ~0xFFFu), 4096, PROT_READ | PROT_WRITE,
                   MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
    if (m == MAP_FAILED) {
        perror("mmap at the flash pages");
        return 1;
    }
    a = (volatile uint32_t *)(uintptr_t)PAGE_A;
    b = (volatile uint32_t *)(uintptr_t)PAGE_B;

    name_t n, got;
    erase(a);
    erase(b);
    CHECK(!store_load(&got), "erased pages: no name");
    CHECK(store_stage() == STAGE_NEW, "erased pages: a new badge");
    CHECK(store_progress() == 0, "erased pages: no progress");

    name_set(&n, "ANNE JAN", 8);
    CHECK(store_save(&n), "first save succeeds");
    lock_check("first save", PAGE_A);
    CHECK(a[0] == 0x4E414D45u && a[1] == 8, "magic \"NAME\" and length (%08x %u)", a[0], a[1]);
    CHECK(a[2] == ('A' | 'N' << 8 | 'N' << 16 | (uint32_t)'E' << 24), "characters packed little-endian (%08x)", a[2]);
    CHECK(a[60] == 1, "first page is number 1 (%u)", a[60]);
    CHECK(b[0] == 0xFFFFFFFFu, "the other page is not touched");
    CHECK(store_load(&got) && strcmp(got.s, "ANNE JAN") == 0 && got.len == 8, "name loads back");
    CHECK(store_save(&n), "saving the same name again succeeds");
    CHECK(fake_flash.KEYR == 0 && fake_flash.CTLR == 0, "saving the same name again does not erase a page");

    CHECK(store_set_progress(0x0b6d), "progress saves");
    lock_check("progress", PAGE_B);
    CHECK(b[60] == 2 && b[62] == 0x50520b6du, "second save goes to the other page, number 2, progress in word 62");
    CHECK(a[1] == 8 && a[60] == 1, "the first page is left as it was");
    CHECK(store_set_stage(STAGE_TESTED), "stage saves");
    lock_check("stage", PAGE_A);
    CHECK(a[60] == 3 && a[62] == 0x50520b6du && a[63] == 0x54455354u, "third save back on the first page (%u %08x %08x)",
          a[60], a[62], a[63]);
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
    CHECK(store_stage() == STAGE_NEW, "stage NEW");
    fake_flash.CTLR = fake_flash.KEYR = fake_flash.MODEKEYR = fake_flash.ADDR = fake_flash.STATR = 0;

    /* power lost halfway through a save: the newest page fails its check */
    volatile uint32_t *newest = a[60] > b[60] ? a : b, *older = newest == a ? b : a;
    name_set(&n, "OLD", 3);
    store_save(&n);
    CHECK(older[1] == 3, "the save went to the older page");
    name_set(&n, "TORN", 4);
    store_save(&n);
    volatile uint32_t *torn = older == a ? b : a;
    CHECK(torn[1] == 4, "and the next one to the other page");
    torn[40] = 0; /* a word that never got programmed */
    CHECK(store_load(&got) && strcmp(got.s, "OLD") == 0, "a page that fails its check falls back to the previous one");
    erase(torn);
    fake_flash.STATR = 0; /* the fake keeps the 1s written to clear its flags */
    CHECK(store_load(&got) && strcmp(got.s, "OLD") == 0, "an erased page falls back to the previous one");
    CHECK(store_save(&n) && torn[1] == 4 && torn[0] == 0x4E414D45u, "the next save rewrites the broken page");
    CHECK(store_load(&got) && strcmp(got.s, "TORN") == 0, "and is the current one");

    /* sequence numbers wrap */
    erase(a);
    erase(b);
    name_set(&n, "WRAP", 4);
    store_save(&n);
    a[60] = 0xFFFFFFFFu;
    reseal(a);
    name_set(&n, "PAST", 4);
    store_save(&n);
    CHECK(b[60] == 0 && store_load(&got) && strcmp(got.s, "PAST") == 0, "number 0 is newer than 0xFFFFFFFF");

    /* the controller reports an error */
    fake_flash.STATR = 0x10;
    name_set(&n, "FAIL", 4);
    CHECK(!store_save(&n), "a write-protection error fails the save");
    fake_flash.STATR = 0;

    /* a name-less page (progress first) */
    erase(a);
    erase(b);
    store_set_progress(0x0001);
    CHECK(!store_load(&got) && store_progress() == 1, "progress without a name: no name loads");

    erase(b);
    name_set(&n, "ABC", 3);
    store_save(&n);
    volatile uint32_t *p = a[1] == 3 ? a : b;
    p[1] = NAME_MAX + 1;
    reseal(p);
    CHECK(!store_load(&got), "length over NAME_MAX refused");
    p[1] = 0;
    reseal(p);
    CHECK(!store_load(&got), "length 0 refused");
    p[1] = 3;
    p[0] = 0x4E414D44u;
    reseal(p);
    CHECK(!store_load(&got), "wrong magic refused");
    p[0] = 0x4E414D45u;
    p[2] = 'a' | 0x01u << 8 | (uint32_t)'~' << 16;
    reseal(p);
    CHECK(store_load(&got) && strcmp(got.s, "A") == 0, "junk characters cleaned up like any name (\"%s\")", got.s);
    p[62] = 0x12340b6du;
    reseal(p);
    CHECK(store_progress() == 0, "progress without the \"PR\" tag is ignored");
    p[63] = 0x54455355u;
    reseal(p);
    CHECK(store_stage() == STAGE_NEW, "an unknown stage word is a new badge");
    TEST_END("store");
}
