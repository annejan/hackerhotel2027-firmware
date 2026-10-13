/* The sponsor reel (C firmware; same as ../../src/sponsors.rs): "THANKS
 * TO", then the sponsors' logos, with the arms in their colours. It plays
 * once, on the first boot after the factory test (store.h keeps that in
 * flash); `sponsors` on the UART console plays it again. Any tap skips it.
 *
 * The logos are greyscale bitmaps in logos.h, made from assets/sponsors by
 * ../../tools/sim/logo.py. One that fits the grid stands in the middle; a
 * wider one scrolls by like text, and one taller than the grid pans from
 * its top to its bottom meanwhile (the ALLNET globe). */
#pragma once
#include <stdbool.h>
#include <stdint.h>

void sponsors_start(void);
/* false once the reel is over */
bool sponsors_render(void);
