/* Badge-to-badge infrared: the packet format and the pulse decoder,
 * independent of the hardware (ir.c on the badge; the emulator passes
 * whole packets). Host-testable.
 *
 * On air it is NEC-style pulse-distance coding on a 38 kHz carrier:
 *   header  9 ms mark, 4.5 ms space
 *   bit     560 us mark, then 560 us space (0) or 1690 us space (1),
 *           least significant bit of each byte first
 *   end     one more 560 us mark, so the last space has an end
 * A mark is carrier on; the receiver's output is low during a mark.
 *
 * A packet is: type, length n, n data bytes, CRC-8 (polynomial 0x07) over
 * all the bytes before it. A name of 24 characters is 27 bytes, about
 * 0.26 s on air. */
#pragma once
#include <stdbool.h>
#include <stdint.h>

#define IR_HDR_MARK_US 9000
#define IR_HDR_SPACE_US 4500
#define IR_BIT_MARK_US 560
#define IR_ZERO_SPACE_US 560
#define IR_ONE_SPACE_US 1690

#define IR_DATA_MAX 24 /* data bytes in one packet */
#define IR_PACKET_MAX (IR_DATA_MAX + 3)

/* packet types */
enum {
    IR_HELLO = 'H', /* data = the sender's name; please reply */
    IR_REPLY = 'R', /* data = the sender's name, answering a HELLO */
    IR_TEST = 'T',  /* the bring-up test's loopback packet */
};

/* builds a packet in out[IR_PACKET_MAX]; returns its length, or 0 when
 * len is more than IR_DATA_MAX */
int ir_pack(uint8_t type, const uint8_t *data, int len, uint8_t *out);

typedef struct {
    uint8_t state, bits, byte, n, want;
    uint8_t buf[IR_PACKET_MAX];
} ir_dec_t;

void ir_dec_reset(ir_dec_t *d);
/* feed one interval as the receiver saw it: a mark (carrier) or a space,
 * and how long it lasted. Returns the packet length once a whole packet
 * with a good CRC has arrived (type in buf[0], length in buf[1], data from
 * buf[2]), else 0. Anything that does not fit starts the search again. */
int ir_dec_feed(ir_dec_t *d, bool mark, uint32_t us);
