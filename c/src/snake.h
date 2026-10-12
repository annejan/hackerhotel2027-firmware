#pragma once
#include <stdbool.h>
#include "events.h"

/* Snake on the 17 x 16 grid; the octagon's edge is the wall. Attract mode
 * plays by itself until T4 starts a game; in a game T1 turns left and T4
 * turns right (on touch-down). The arms fill green with the score. */
void snake_reset(void);
/* returns true when the event was used by the game */
bool snake_event(event_t ev, uint8_t arg);
bool snake_playing(void);
void snake_render(void);
