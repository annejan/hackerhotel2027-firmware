/* The only libc functions the compiler may emit calls to. Built without a
 * C library: the installed riscv64-elf toolchain has no rv32 newlib. */
#include <stddef.h>

void *memset(void *dst, int c, size_t n)
{
    unsigned char *d = dst;
    while (n--)
        *d++ = (unsigned char)c;
    return dst;
}

void *memcpy(void *restrict dst, const void *restrict src, size_t n)
{
    unsigned char *d = dst;
    const unsigned char *s = src;
    while (n--)
        *d++ = *s++;
    return dst;
}
