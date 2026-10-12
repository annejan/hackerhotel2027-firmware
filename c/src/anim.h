#pragma once
#include <stdint.h>

#define FPS 30

void anim_init(void);
/* render one frame into fb[] */
void anim_render(void);
uint8_t anim_brightness(void);
