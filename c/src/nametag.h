#pragma once
#include <stdint.h>

/* longest name, in characters */
#define NAME_MAX 24

/* a name: upper case, only characters the font has, no trailing spaces;
 * NUL-terminated so it can be drawn as text */
typedef struct {
    char s[NAME_MAX + 1];
    uint8_t len;
} name_t;

/* from len bytes of s (more than NAME_MAX are cut off) */
void name_set(name_t *n, const char *s, int len);
/* the build-time default: make NAME="..." or "YOUR NAME" */
void name_default(name_t *n);

/* name tag: short names stand still, long ones scroll */
void nametag_name(uint32_t t, const name_t *name);
/* Hacker Hotel 2027: logo, then the full name scrolls by */
void nametag_hotel(uint32_t t);
