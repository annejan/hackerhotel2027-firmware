# Host tests for the C firmware: built with the PC's compiler, under
# AddressSanitizer and UndefinedBehaviorSanitizer.
#
#   make -C c test            (from the repo root; or `make test` in c/)
#   make -C c/test SOAK=1000000
#
# Hardware-independent modules link against stubs.c (or emu/emu.c for the
# soak). The drivers (buttons, ir, ledpwm, store) build against
# fake/ch643.h, plain structs in place of the peripherals.

C       ?= ..
SRC     := $(C)/src
OUT     ?= $(C)/build/test
HOSTCC  ?= cc
SOAK    ?= 200000
SAN     ?= -fsanitize=address,undefined -fno-sanitize-recover=all
# the drivers' interrupt handlers: an ordinary function on the PC
CFLAGS  := -std=gnu11 -O1 -g -Wall -Wextra -Wno-clobbered $(SAN) -Dinterrupt=unused \
           -I. -Ifake -I$(SRC) -I$(C)/../tools/emu

PURE := app anim console editor frame generated game irproto proto selftest factory \
        sponsors challenge nametag
PURE_SRCS := $(PURE:%=$(SRC)/%.c) $(wildcard $(SRC)/games/*.c)
HDRS := $(wildcard $(SRC)/*.h) $(wildcard *.h fake/*.h)

TESTS := buttons ir ledpwm store app challenge soak

all: $(TESTS:%=$(OUT)/test_%)
	@set -e; for t in $(TESTS); do \
	    if [ $$t = soak ]; then $(OUT)/test_$$t $(SOAK) 2>/dev/null; else $(OUT)/test_$$t; fi; done

$(OUT):
	mkdir -p $@

$(OUT)/test_buttons: test_buttons.c $(SRC)/buttons.c fake/fake.c $(HDRS) | $(OUT)
	$(HOSTCC) $(CFLAGS) test_buttons.c $(SRC)/buttons.c fake/fake.c -o $@
$(OUT)/test_ir: test_ir.c $(SRC)/ir.c $(SRC)/irproto.c fake/fake.c $(HDRS) | $(OUT)
	$(HOSTCC) $(CFLAGS) test_ir.c $(SRC)/ir.c $(SRC)/irproto.c fake/fake.c -o $@
# includes ledpwm.c itself, to see the row buffers
$(OUT)/test_ledpwm: test_ledpwm.c $(SRC)/ledpwm.c $(SRC)/frame.c $(SRC)/generated.c fake/fake.c $(HDRS) | $(OUT)
	$(HOSTCC) $(CFLAGS) test_ledpwm.c $(SRC)/frame.c $(SRC)/generated.c fake/fake.c -o $@
$(OUT)/test_store: test_store.c $(SRC)/store.c $(PURE_SRCS) stubs.c fake/fake.c $(HDRS) | $(OUT)
	$(HOSTCC) $(CFLAGS) -Wno-int-to-pointer-cast test_store.c $(SRC)/store.c $(SRC)/nametag.c $(SRC)/frame.c $(SRC)/generated.c fake/fake.c -o $@
$(OUT)/test_app: test_app.c stubs.c $(PURE_SRCS) $(HDRS) | $(OUT)
	$(HOSTCC) $(CFLAGS) test_app.c stubs.c $(PURE_SRCS) -o $@
$(OUT)/test_challenge: test_challenge.c stubs.c $(PURE_SRCS) $(HDRS) | $(OUT)
	$(HOSTCC) $(CFLAGS) test_challenge.c stubs.c $(PURE_SRCS) -o $@
$(OUT)/test_soak: test_soak.c $(C)/emu/emu.c $(PURE_SRCS) $(HDRS) | $(OUT)
	$(HOSTCC) $(CFLAGS) test_soak.c $(C)/emu/emu.c $(PURE_SRCS) -o $@

clean:
	rm -rf $(OUT)

.PHONY: all clean
