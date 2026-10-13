#include <string.h>
#include "app.h"
#include "console.h"
#include "log.h"

/* "word rest": true when line starts with word, followed by the end or a
 * space; *rest points after the spaces */
static int is(const char *line, const char *word, const char **rest)
{
    while (*word)
        if (*line++ != *word++)
            return 0;
    if (*line && *line != ' ')
        return 0;
    while (*line == ' ')
        line++;
    *rest = line;
    return 1;
}

static int number(const char *s, int *out)
{
    int v = 0;
    if (!*s)
        return 0;
    for (; *s; s++) {
        if (*s < '0' || *s > '9' || v > 1000)
            return 0;
        v = v * 10 + (*s - '0');
    }
    *out = v;
    return 1;
}

static void info(void)
{
    name_t name;
    app_name(&name);
    log_str("c firmware, name \"");
    log_str(name.s);
    log_str("\", showing ");
    log_u32(app_item());
    log_str(" ");
    log_str(app_label(app_item()));
    log_str(", level ");
    log_u32(app_level());
    log_line(app_charging() ? ", charging" : "");
}

void console_line(const char *line)
{
    const char *arg;
    int n;
    while (*line == ' ')
        line++;
    if (is(line, "help", &arg)) {
        log_line("info | name TEXT | show N | level N | hello | test | factory [skip] | sponsors");
        for (int i = 0; i < app_items(); i++) {
            log_str("  ");
            log_u32((uint32_t)i);
            log_str(" ");
            log_line(app_label((uint8_t)i));
        }
    } else if (is(line, "info", &arg)) {
        info();
    } else if (is(line, "name", &arg)) {
        name_t name;
        name_set(&name, arg, (int)strlen(arg));
        app_set_name(&name);
        info();
    } else if (is(line, "show", &arg) && number(arg, &n) && n < 256 && app_set_item((uint8_t)n)) {
        info();
    } else if (is(line, "level", &arg) && number(arg, &n) && n < app_levels()) {
        app_set_level((uint8_t)n);
        info();
    } else if (is(line, "test", &arg)) {
        app_selftest();
    } else if (is(line, "factory", &arg)) {
        if (is(arg, "skip", &arg))
            app_factory_skip();
        else
            app_factory();
    } else if (is(line, "sponsors", &arg)) {
        app_sponsors();
    } else if (is(line, "hello", &arg)) {
        if (!app_ir_hello())
            log_line("ir: busy");
    } else {
        log_line("? try help");
    }
}
