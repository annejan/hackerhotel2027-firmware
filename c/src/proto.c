/* The badge's USB protocol, see proto.h. */
#include <string.h>
#include "app.h"
#include "proto.h"

enum { INFO = 1, GET_NAME, SET_NAME, GET_STATE, SET_ITEM, SET_LEVEL, GET_LABEL, ERR = 0xFF };

static int error(uint8_t cmd, uint8_t *out)
{
    out[0] = ERR;
    out[1] = cmd;
    return 2;
}

static int state(uint8_t *out)
{
    out[0] = GET_STATE;
    out[1] = app_item();
    out[2] = app_level();
    out[3] = app_charging();
    return 4;
}

static int text(uint8_t *out, int at, const char *s, int len)
{
    out[at] = (uint8_t)len;
    memcpy(out + at + 1, s, (size_t)len);
    return at + 1 + len;
}

int proto_handle(const uint8_t *in, int len, uint8_t *out)
{
    if (len < 1)
        return error(0, out);
    uint8_t cmd = in[0];
    name_t name;
    switch (cmd) {
    case INFO:
        out[0] = INFO;
        out[1] = PROTO_VERSION;
        out[2] = app_items();
        out[3] = app_levels();
        out[4] = NAME_MAX;
        out[5] = 'c';
        return 6;
    case SET_NAME:
        if (len < 2 || in[1] > NAME_MAX || len < 2 + in[1])
            return error(cmd, out);
        name_set(&name, (const char *)in + 2, in[1]);
        app_set_name(&name);
        /* reply with the name as it is now, cleaned up */
        __attribute__((fallthrough));
    case GET_NAME:
        app_name(&name);
        out[0] = cmd;
        return text(out, 1, name.s, name.len);
    case SET_ITEM:
        if (len < 2 || !app_set_item(in[1]))
            return error(cmd, out);
        return state(out);
    case SET_LEVEL:
        if (len < 2 || in[1] >= app_levels())
            return error(cmd, out);
        app_set_level(in[1]);
        return state(out);
    case GET_STATE:
        return state(out);
    case GET_LABEL: {
        if (len < 2 || in[1] >= app_items())
            return error(cmd, out);
        const char *s = app_label(in[1]);
        int n = (int)strlen(s);
        out[0] = GET_LABEL;
        out[1] = in[1];
        return text(out, 2, s, n > PROTO_PACKET - 3 ? PROTO_PACKET - 3 : n);
    }
    default:
        return error(cmd, out);
    }
}
