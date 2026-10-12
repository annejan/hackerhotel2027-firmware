/* The badge's USB protocol, independent of the USB driver: usb.c on the
 * badge and the emulator (emu/emu.c) both feed it. The browser page is
 * ../../docs/webusb/index.html.
 *
 * One command per bulk OUT packet, one reply per bulk IN packet, at most
 * 64 bytes each. The reply starts with the command byte; 0xFF = unknown
 * command or bad argument (then the command byte follows).
 *
 *   0x01 INFO                -> 01 version items levels name_max firmware('c')
 *   0x02 GET NAME            -> 02 len chars...
 *   0x03 SET NAME len chars  -> 03 len chars...   (saved to flash)
 *   0x04 GET STATE           -> 04 item level charging
 *   0x05 SET ITEM i          -> 04 item level charging
 *   0x06 SET LEVEL l         -> 04 item level charging
 *   0x07 GET LABEL i         -> 07 i len chars...
 */
#pragma once
#include <stdint.h>

#define PROTO_VERSION 1
#define PROTO_PACKET 64

/* handle one command; returns the reply length (1..PROTO_PACKET) */
int proto_handle(const uint8_t *in, int len, uint8_t *out);
