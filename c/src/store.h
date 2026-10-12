#pragma once
#include <stdbool.h>
#include "nametag.h"

/* the saved name, if there is one */
bool store_load(name_t *out);
void store_save(const name_t *name);
