#pragma once
#include <stdbool.h>
#include "nametag.h"

/* the saved name, if there is one */
bool store_load(name_t *out);
void store_save(const name_t *name);

/* How far the badge has got since it was flashed: the factory test
 * (factory.h) runs until it passes, the sponsor reel (sponsors.h) plays on
 * the boot after that. Kept in the same page as the name. */
typedef enum { STAGE_NEW, STAGE_TESTED, STAGE_DONE } stage_t;
stage_t store_stage(void);
void store_set_stage(stage_t stage);

/* the challenges' progress (challenge.h), packed; 0 when none is kept */
uint16_t store_progress(void);
void store_set_progress(uint16_t packed);
