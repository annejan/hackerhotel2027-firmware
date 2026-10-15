/* USB device for the WebUSB protocol (proto.h). NOT TESTED ON A CH643.
 *
 * Based on fsusb.c from ch32fun's CH32X035 USB device example (MIT,
 * CNLohr et al.), in the version from the badge team's webusb_vendor test,
 * which enumerated and ran bulk IN/OUT on a real CH32X035. The CH643's USB
 * block sits at the same address with the same registers, and WCH's CH643
 * USB device example sets up PC16/PC17 the same way, so this should carry
 * over; it is still unproven on this chip.
 *
 * Polled from the main loop instead of an interrupt (the C firmware has
 * none): while an event waits to be handled the USB block answers NAK
 * (UC_INT_BUSY), and the host simply retries.
 *
 * EP0 control, EP1 bulk IN, EP2 bulk OUT. A BOS descriptor with a
 * Microsoft OS 2.0 descriptor set makes Windows bind WinUSB by itself, so
 * browsers can use WebUSB there without installing a driver. */
#include <stdbool.h>
#include <string.h>
#include "ch643.h"
#include "proto.h"
#include "usb.h"

/* pid.codes test PID, as in the X035 WebUSB test. A real badge needs its
 * own VID/PID (badge.team's 16D0 range, or a pid.codes allocation). */
#define VID 0x1209
#define PID 0xD035
#define MS_VENDOR_CODE 0x20 /* bRequest for the Microsoft OS 2.0 set */

/* USB block bits (ch643_usb.h, WCH's USB library header) */
#define UC_DEV_PU_EN 0x20
#define UC_INT_BUSY 0x08
#define UC_RESET_SIE 0x04
#define UC_CLR_ALL 0x02
#define UC_DMA_EN 0x01
#define UD_PD_DIS 0x80
#define UD_PORT_EN 0x01
#define UIE_SUSPEND 0x04
#define UIE_TRANSFER 0x02
#define UIE_BUS_RST 0x01
#define UIF_TRANSFER 0x02
#define UIF_BUS_RST 0x01
#define UIS_TOG_OK 0x40
#define UIS_TOKEN_MASK 0x30
#define UIS_TOKEN_OUT 0x00
#define UIS_TOKEN_IN 0x20
#define UIS_TOKEN_SETUP 0x30
#define UIS_ENDP_MASK 0x0F
#define UDA_GP_BIT 0x80
#define UEP1_TX_EN 0x40
#define UEP2_RX_EN 0x08
#define T_TOG (1 << 6)
#define T_RES_MASK (3 << 0)
#define T_RES_ACK (0 << 0)
#define T_RES_NAK (1 << 1)
#define T_RES_STALL (3 << 0)
#define R_TOG (1 << 7)
#define R_RES_MASK (3 << 2)
#define R_RES_ACK (0 << 2)
#define R_RES_NAK (1 << 3)
#define R_RES_STALL (3 << 2)

/* AFIO_CTLR: PC16/PC17 as USB, pull-up on D+ */
#define UDP_PUE_10K 0x08

/* the 96-bit unique ID, as on the CH32X035 (GUESS for the CH643) */
#define UID ((const volatile uint32_t *)0x1FFFF7E8)

static const uint8_t device_desc[] = {
    18, 1,       /* device */
    0x10, 0x02,  /* USB 2.1: needed for the BOS descriptor */
    0, 0, 0, 64, /* class in the interface; EP0 64 bytes */
    VID & 0xFF, VID >> 8, PID & 0xFF, PID >> 8,
    0x00, 0x01, /* release 1.00 */
    1, 2, 3,    /* manufacturer, product, serial strings */
    1,
};

static const uint8_t config_desc[] = {
    9, 2, 32, 0, 1, 1, 0, 0x80, 250, /* bus powered, up to 500 mA (it charges) */
    9, 4, 0, 0, 2, 0xFF, 0x00, 0x00, 0, /* one vendor interface, 2 endpoints */
    7, 5, 0x81, 2, 64, 0, 0,             /* EP1 bulk IN */
    7, 5, 0x02, 2, 64, 0, 0,             /* EP2 bulk OUT */
};

#define MS_SET_LEN 162
static const uint8_t bos_desc[] = {
    5, 15, 33, 0, 1, /* BOS, total 33 bytes, one capability */
    28, 16, 5, 0,    /* platform capability: Microsoft OS 2.0 */
    0xDF, 0x60, 0xDD, 0xD8, 0x89, 0x45, 0xC7, 0x4C,
    0x9C, 0xD2, 0x65, 0x9D, 0x9E, 0x64, 0x8A, 0x9F,
    0x00, 0x00, 0x03, 0x06, /* Windows 8.1 and later */
    MS_SET_LEN, 0, MS_VENDOR_CODE, 0,
};

#define U16(s) s, 0 /* one UTF-16LE character from ASCII */
static const uint8_t ms_set[] = {
    10, 0, 0, 0, 0x00, 0x00, 0x03, 0x06, MS_SET_LEN, 0, /* set header */
    20, 0, 3, 0, 'W', 'I', 'N', 'U', 'S', 'B', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, /* compatible ID */
    132, 0, 4, 0, 7, 0, 42, 0, /* registry property, REG_MULTI_SZ */
    U16('D'), U16('e'), U16('v'), U16('i'), U16('c'), U16('e'), U16('I'), U16('n'), U16('t'),
    U16('e'), U16('r'), U16('f'), U16('a'), U16('c'), U16('e'), U16('G'), U16('U'), U16('I'),
    U16('D'), U16('s'), U16(0),
    80, 0,
    U16('{'), U16('8'), U16('F'), U16('6'), U16('D'), U16('5'), U16('B'), U16('3'), U16('C'),
    U16('-'), U16('2'), U16('0'), U16('2'), U16('7'), U16('-'), U16('4'), U16('E'), U16('1'),
    U16('A'), U16('-'), U16('9'), U16('C'), U16('5'), U16('5'), U16('-'), U16('4'), U16('8'),
    U16('4'), U16('8'), U16('4'), U16('8'), U16('3'), U16('7'), U16('A'), U16('0'), U16('2'),
    U16('7'), U16('}'), U16(0), U16(0),
};
_Static_assert(sizeof ms_set == MS_SET_LEN, "Microsoft OS 2.0 descriptor set length");
_Static_assert(sizeof bos_desc == 33, "BOS length");
_Static_assert(sizeof config_desc == 32, "configuration length");

static const uint8_t lang_desc[] = {4, 3, 0x09, 0x04};
static const char *const STRINGS[] = {"badge.team", "Hacker Hotel 2027 starfish"};

static uint8_t ep0_buf[64] __attribute__((aligned(4)));
static uint8_t ep1_buf[64] __attribute__((aligned(4)));
static uint8_t ep2_buf[64] __attribute__((aligned(4)));

static struct {
    uint8_t text[2 + 2 * 26]; /* the string descriptor being sent */
    const uint8_t *tx;        /* rest of a control IN transfer */
    uint16_t left;
    uint8_t request, addr, config;
    bool ep1_busy;
    uint8_t reply[PROTO_PACKET], reply_len; /* waiting for EP1 to be free */
} u;

static void endpoints(void)
{
    USBFSD->UEP4_1_MOD = UEP1_TX_EN;
    USBFSD->UEP2_3_MOD = UEP2_RX_EN;
    USBFSD->UEP567_MOD = 0;
    USBFSD->UEP0_DMA = (uint32_t)(uintptr_t)ep0_buf;
    USBFSD->UEP1_DMA = (uint32_t)(uintptr_t)ep1_buf;
    USBFSD->UEP2_DMA = (uint32_t)(uintptr_t)ep2_buf;
    USBFSD->UEP0_CTRL_H = R_RES_ACK | T_RES_NAK;
    USBFSD->UEP1_CTRL_H = T_RES_NAK;
    USBFSD->UEP2_CTRL_H = R_RES_ACK;
    u.ep1_busy = false;
    u.reply_len = 0;
}

void usb_init(void)
{
    RCC->AHBPCENR |= RCC_USBFS;
    RCC->APB2PCENR |= RCC_AFIOEN | RCC_IOPCEN;

    /* PC16 (D-) floating input, PC17 (D+) input with pull-up */
    GPIOC->CFGXR = (GPIOC->CFGXR & ~0xFFu) | 0x84;
    GPIOC->BSXR = 1u << 1;
    /* Always the 5 V setup: the PHY behind its 3.3 V regulator, 10k on D+.
     * VDD only comes from USB through the diode (about 4.7 V), so whenever
     * there is a host to talk to, VDD is above 4 V. Choosing by VDD at boot
     * (WCH's PWR_VDD_SupplyVoltage()) goes wrong when the badge starts on
     * its battery and USB is plugged in later: the PHY would then drive D+
     * at VDD, above the 3.6 V USB allows. */
    uint32_t ctlr = AFIO->CTLR & ~(AFIO_CTLR_UDP_PUE | AFIO_CTLR_UDM_PUE | AFIO_CTLR_USB_PHY_V33);
    AFIO->CTLR = ctlr | AFIO_CTLR_USB_IOEN | UDP_PUE_10K;

    USBFSD->BASE_CTRL = UC_RESET_SIE | UC_CLR_ALL;
    USBFSD->BASE_CTRL = 0;
    endpoints();
    USBFSD->DEV_ADDR = 0;
    USBFSD->INT_EN = UIE_SUSPEND | UIE_TRANSFER | UIE_BUS_RST; /* flags only; no interrupt */
    USBFSD->BASE_CTRL = UC_DEV_PU_EN | UC_INT_BUSY | UC_DMA_EN;
    USBFSD->INT_FG = 0xFF;
    USBFSD->UDEV_CTRL = UD_PD_DIS | UD_PORT_EN;
}

/* a string descriptor from ASCII; 3 = the serial number, from the chip ID */
static const uint8_t *string_desc(int i, uint16_t *len)
{
    char serial[25];
    const char *s;
    if (i == 3) {
        static const char HEX[] = "0123456789ABCDEF";
        for (int k = 0; k < 24; k++)
            serial[k] = HEX[UID[k / 8] >> (28 - 4 * (k % 8)) & 0xF];
        serial[24] = 0;
        s = serial;
    } else {
        s = STRINGS[i - 1];
    }
    int n = (int)strlen(s);
    if (n > 26)
        n = 26;
    u.text[0] = (uint8_t)(2 + 2 * n);
    u.text[1] = 3;
    for (int k = 0; k < n; k++) {
        u.text[2 + 2 * k] = (uint8_t)s[k];
        u.text[3 + 2 * k] = 0;
    }
    *len = u.text[0];
    return u.text;
}

/* start a control IN transfer: the first packet now, the rest on each IN */
static void control_in(const uint8_t *data, uint16_t len, uint16_t wlength)
{
    if (len > wlength)
        len = wlength;
    uint16_t n = len > 64 ? 64 : len;
    memcpy(ep0_buf, data, n);
    u.left = len - n;
    u.tx = u.left ? data + n : 0;
    /* (no zero-length packet: none of these lengths is a multiple of 64) */
    USBFSD->UEP0_TX_LEN = n;
    USBFSD->UEP0_CTRL_H = T_TOG | T_RES_ACK; /* DATA1 */
}

static void stall(void)
{
    USBFSD->UEP0_CTRL_H = T_TOG | T_RES_STALL | R_TOG | R_RES_STALL;
}

static void setup(void)
{
    uint8_t type = ep0_buf[0], req = ep0_buf[1];
    uint16_t value = ep0_buf[2] | ep0_buf[3] << 8;
    uint16_t index = ep0_buf[4] | ep0_buf[5] << 8;
    uint16_t wlength = ep0_buf[6] | ep0_buf[7] << 8;
    static uint8_t small[2];
    u.request = req;
    u.tx = 0;
    u.left = 0;
    USBFSD->UEP0_CTRL_H = T_TOG | T_RES_NAK | R_TOG | R_RES_NAK;

    if ((type & 0x60) == 0x40) { /* vendor */
        if (req == MS_VENDOR_CODE && index == 7 && (type & 0x80))
            control_in(ms_set, sizeof ms_set, wlength);
        else
            stall();
        return;
    }
    if ((type & 0x60) != 0) {
        stall();
        return;
    }
    switch (req) {
    case 6: { /* GET_DESCRIPTOR */
        const uint8_t *d = 0;
        uint16_t len = 0;
        switch (value >> 8) {
        case 1: d = device_desc, len = sizeof device_desc; break;
        case 2: d = config_desc, len = sizeof config_desc; break;
        case 3:
            if ((value & 0xFF) == 0)
                d = lang_desc, len = sizeof lang_desc;
            else if ((value & 0xFF) <= 3)
                d = string_desc(value & 0xFF, &len);
            break;
        case 15: d = bos_desc, len = sizeof bos_desc; break;
        }
        if (!d)
            stall();
        else
            control_in(d, len, wlength);
        return;
    }
    case 5: /* SET_ADDRESS: takes effect after the status stage */
        u.addr = value & 0x7F;
        break;
    case 9: /* SET_CONFIGURATION */
        u.config = value & 0xFF;
        endpoints();
        break;
    case 8: /* GET_CONFIGURATION */
        small[0] = u.config;
        control_in(small, 1, wlength);
        return;
    case 0: /* GET_STATUS */
        small[0] = small[1] = 0;
        control_in(small, 2, wlength);
        return;
    case 10: /* GET_INTERFACE */
        small[0] = 0;
        control_in(small, 1, wlength);
        return;
    case 1:  /* CLEAR_FEATURE (endpoint halt) */
    case 11: /* SET_INTERFACE */
        if (req == 1 && (type & 0x1F) == 2) {
            if ((index & 0x0F) == 1)
                USBFSD->UEP1_CTRL_H = T_RES_NAK, u.ep1_busy = false;
            else if ((index & 0x0F) == 2)
                USBFSD->UEP2_CTRL_H = R_RES_ACK;
        }
        break;
    default:
        stall();
        return;
    }
    /* no data stage: answer the status stage with an empty DATA1 packet */
    USBFSD->UEP0_TX_LEN = 0;
    USBFSD->UEP0_CTRL_H = T_TOG | T_RES_ACK;
}

static void ep0_in(void)
{
    if (u.left == 0 && u.tx == 0) {
        if (u.request == 5) /* SET_ADDRESS */
            USBFSD->DEV_ADDR = (USBFSD->DEV_ADDR & UDA_GP_BIT) | u.addr;
        USBFSD->UEP0_CTRL_H = R_TOG | R_RES_ACK; /* ready for the next request */
        return;
    }
    uint16_t n = u.left > 64 ? 64 : u.left;
    memcpy(ep0_buf, u.tx, n);
    u.left -= n;
    u.tx = u.left ? u.tx + n : 0;
    USBFSD->UEP0_TX_LEN = n;
    USBFSD->UEP0_CTRL_H ^= T_TOG;
}

/* hand the waiting reply to EP1 once it is free; until then EP2 says NAK,
 * so the host cannot send the next command before taking this reply */
static void send_reply(void)
{
    if (!u.reply_len || u.ep1_busy)
        return;
    memcpy(ep1_buf, u.reply, u.reply_len);
    USBFSD->UEP1_TX_LEN = u.reply_len;
    USBFSD->UEP1_CTRL_H = (USBFSD->UEP1_CTRL_H & ~T_RES_MASK) | T_RES_ACK;
    u.ep1_busy = true;
    u.reply_len = 0;
    USBFSD->UEP2_CTRL_H = (USBFSD->UEP2_CTRL_H & ~R_RES_MASK) | R_RES_ACK;
}

void usb_poll(void)
{
    uint8_t fg = USBFSD->INT_FG;
    if (fg & UIF_TRANSFER) {
        uint8_t st = USBFSD->INT_ST;
        uint8_t ep = st & UIS_ENDP_MASK;
        switch (st & UIS_TOKEN_MASK) {
        case UIS_TOKEN_SETUP:
            setup();
            break;
        case UIS_TOKEN_IN:
            if (ep == 0) {
                ep0_in();
            } else if (ep == 1) {
                USBFSD->UEP1_CTRL_H = ((USBFSD->UEP1_CTRL_H & ~T_RES_MASK) | T_RES_NAK) ^ T_TOG;
                u.ep1_busy = false;
            }
            break;
        case UIS_TOKEN_OUT:
            if (!(st & UIS_TOG_OK))
                break;
            if (ep == 0) {
                /* status stage after an IN transfer */
                USBFSD->UEP0_TX_LEN = 0;
                USBFSD->UEP0_CTRL_H = T_TOG | T_RES_ACK;
            } else if (ep == 2) {
                u.reply_len = (uint8_t)proto_handle(ep2_buf, USBFSD->RX_LEN, u.reply);
                USBFSD->UEP2_CTRL_H = ((USBFSD->UEP2_CTRL_H & ~R_RES_MASK) | R_RES_NAK) ^ R_TOG;
            }
            break;
        }
    } else if (fg & UIF_BUS_RST) {
        u.addr = u.config = 0;
        USBFSD->DEV_ADDR = 0;
        endpoints();
    }
    if (fg)
        USBFSD->INT_FG = fg;
    send_reply();
}
