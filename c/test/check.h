/* CHECK(cond, fmt, ...): count and print a failure; TEST_END returns the
 * exit status. Quiet when everything passes, so `make test` output stays
 * short. */
#pragma once
#include <stdio.h>

static int check_failed, check_run;
#define CHECK(cond, ...)                                       \
    do {                                                       \
        check_run++;                                           \
        if (!(cond)) {                                         \
            check_failed++;                                    \
            printf("%s:%d: FAIL ", __FILE__, __LINE__);        \
            printf(__VA_ARGS__);                               \
            putchar('\n');                                     \
        }                                                      \
    } while (0)
#define TEST_END(name)                                                          \
    do {                                                                        \
        printf("%-10s %d checks, %d failed\n", name, check_run, check_failed); \
        return check_failed != 0;                                               \
    } while (0)
