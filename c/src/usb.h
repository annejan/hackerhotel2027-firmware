#pragma once

/* USB device (WebUSB protocol, see proto.h); untested on a CH643 */
void usb_init(void);
/* call as often as possible from the main loop */
void usb_poll(void);
