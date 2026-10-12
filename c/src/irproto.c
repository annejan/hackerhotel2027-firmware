#include "irproto.h"

enum { IDLE, HDR_SPACE, BIT_MARK, BIT_SPACE };

static uint8_t crc8(const uint8_t *p, int n)
{
    uint8_t c = 0;
    while (n--) {
        c ^= *p++;
        for (int k = 0; k < 8; k++)
            c = (uint8_t)(c & 0x80 ? c << 1 ^ 0x07 : c << 1);
    }
    return c;
}

int ir_pack(uint8_t type, const uint8_t *data, int len, uint8_t *out)
{
    if (len < 0 || len > IR_DATA_MAX)
        return 0;
    out[0] = type;
    out[1] = (uint8_t)len;
    for (int i = 0; i < len; i++)
        out[2 + i] = data[i];
    out[2 + len] = crc8(out, 2 + len);
    return len + 3;
}

void ir_dec_reset(ir_dec_t *d)
{
    d->state = IDLE;
}

/* receivers stretch marks and shorten spaces by a 100 us or so, and the
 * badge's clock is an RC oscillator: allow plenty */
static bool near(uint32_t us, uint32_t want)
{
    return us > want * 2 / 3 && us < want * 3 / 2;
}

int ir_dec_feed(ir_dec_t *d, bool mark, uint32_t us)
{
    switch (d->state) {
    case IDLE:
        if (mark && near(us, IR_HDR_MARK_US))
            d->state = HDR_SPACE;
        return 0;
    case HDR_SPACE:
        if (!mark && near(us, IR_HDR_SPACE_US)) {
            d->state = BIT_MARK;
            d->bits = d->byte = d->n = 0;
            d->want = 2; /* until the length byte is in */
            return 0;
        }
        break;
    case BIT_MARK:
        if (mark && us > 250 && us < 1000) {
            d->state = BIT_SPACE;
            return 0;
        }
        break;
    case BIT_SPACE:
        if (mark)
            break;
        if (us > 250 && us < 1000)
            d->byte >>= 1;
        else if (us >= 1000 && us < 2400)
            d->byte = (uint8_t)(d->byte >> 1 | 0x80);
        else
            break;
        d->state = BIT_MARK;
        if (++d->bits < 8)
            return 0;
        d->bits = 0;
        d->buf[d->n++] = d->byte;
        if (d->n == 2) {
            if (d->buf[1] > IR_DATA_MAX)
                break;
            d->want = (uint8_t)(d->buf[1] + 3);
        }
        if (d->n < d->want)
            return 0;
        d->state = IDLE;
        return crc8(d->buf, d->n - 1) == d->buf[d->n - 1] ? d->n : 0;
    }
    /* out of step: this interval may be the start of a new header */
    d->state = IDLE;
    return ir_dec_feed(d, mark, us);
}
