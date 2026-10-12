#include "frame.h"

uint8_t fb[SLOTS];

rgb_t hsv(uint8_t h, uint8_t s, uint8_t v)
{
    if (s == 0)
        return (rgb_t){v, v, v};
    uint8_t region = h / 43;
    uint8_t rem = (uint8_t)((h - region * 43) * 6);
    uint8_t p = scale8(v, 255 - s);
    uint8_t q = scale8(v, 255 - scale8(s, rem));
    uint8_t t = scale8(v, 255 - scale8(s, 255 - rem));
    switch (region) {
    case 0: return (rgb_t){v, t, p};
    case 1: return (rgb_t){q, v, p};
    case 2: return (rgb_t){p, v, t};
    case 3: return (rgb_t){p, q, v};
    case 4: return (rgb_t){t, p, v};
    default: return (rgb_t){v, p, q};
    }
}

static uint32_t rng_state = 0x2027u * 0x5EA5u;

uint32_t rng_next(void)
{
    uint32_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    return rng_state = x;
}
