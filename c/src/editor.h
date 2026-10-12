#pragma once
#include <stdint.h>
#include "events.h"
#include "nametag.h"

typedef enum { EDIT_BUSY, EDIT_SAVE, EDIT_CANCEL } edit_t;

/* start editing a copy of name */
void editor_start(const name_t *name);
/* EDIT_SAVE: the new name is in *out */
edit_t editor_event(event_t ev, uint8_t arg, name_t *out);
/* one frame into fb[]; held = pads touched right now. EDIT_CANCEL after a
 * minute untouched */
edit_t editor_render(uint8_t held);
