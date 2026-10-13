/* The sponsor reel (C firmware; same as ../../src/sponsors.rs): "THANKS
 * TO", then every sponsor's logo (when there is one) and name. It plays
 * once, on the first boot after the factory test (store.h keeps that in
 * flash); `sponsors` on the UART console plays it again. Any tap skips it.
 *
 * A logo is GRID_H rows of GRID_W bits (bit c = column c, as MASK), made
 * from a picture with ../../tools/sim/logo.py. */
#pragma once
#include <stdbool.h>
#include <stdint.h>

void sponsors_start(void);
/* false once the reel is over */
bool sponsors_render(void);
