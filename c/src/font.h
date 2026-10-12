/* 5x7 font for the grid scroller. Each glyph is 5 columns, bit 0 = top row.
 * Only the characters the badge needs; anything else renders as a space. */
#pragma once
#include <stdint.h>

#define FONT_W 5
#define FONT_H 7

static inline const uint8_t *font_glyph(char c)
{
    static const uint8_t blank[FONT_W];
    switch (c) {
    case 'A': { static const uint8_t g[FONT_W] = {0x7E, 0x09, 0x09, 0x09, 0x7E}; return g; }
    case 'C': { static const uint8_t g[FONT_W] = {0x3E, 0x41, 0x41, 0x41, 0x22}; return g; }
    case 'E': { static const uint8_t g[FONT_W] = {0x7F, 0x49, 0x49, 0x49, 0x41}; return g; }
    case 'H': { static const uint8_t g[FONT_W] = {0x7F, 0x08, 0x08, 0x08, 0x7F}; return g; }
    case 'K': { static const uint8_t g[FONT_W] = {0x7F, 0x08, 0x14, 0x22, 0x41}; return g; }
    case 'L': { static const uint8_t g[FONT_W] = {0x7F, 0x40, 0x40, 0x40, 0x40}; return g; }
    case 'O': { static const uint8_t g[FONT_W] = {0x3E, 0x41, 0x41, 0x41, 0x3E}; return g; }
    case 'R': { static const uint8_t g[FONT_W] = {0x7F, 0x09, 0x19, 0x29, 0x46}; return g; }
    case 'T': { static const uint8_t g[FONT_W] = {0x01, 0x01, 0x7F, 0x01, 0x01}; return g; }
    case '0': { static const uint8_t g[FONT_W] = {0x3E, 0x51, 0x4D, 0x43, 0x3E}; return g; }
    case '2': { static const uint8_t g[FONT_W] = {0x62, 0x51, 0x49, 0x49, 0x46}; return g; }
    case '7': { static const uint8_t g[FONT_W] = {0x01, 0x71, 0x09, 0x05, 0x03}; return g; }
    case '*': { static const uint8_t g[FONT_W] = {0x14, 0x08, 0x3E, 0x08, 0x14}; return g; }
    case '!': { static const uint8_t g[FONT_W] = {0x00, 0x00, 0x5F, 0x00, 0x00}; return g; }
    default: return blank;
    }
}
