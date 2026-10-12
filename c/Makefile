# C port of the HH2027 starfish firmware, for size comparison with the Rust one.
#
#   make            build build/hh27.elf / .bin (fetches the WCH SDK on first run)
#   make size       section sizes
#   make flash      flash over USB with wchisp (boot ROM, see ../README.md)
#   make emu        build/libhh27.so for the PC emulator (tools/emu)
#
# Uses only startup, linker script, clock init and register headers from
# WCH's SDK (github.com/openwch/ch643); no peripheral library, no libc.
# The SDK is fetched at build time and not committed.

CROSS   ?= riscv-none-elf-
CC      := $(CROSS)gcc
OBJCOPY := $(CROSS)objcopy
SIZE    ?= size

WCH_REPO := https://github.com/openwch/ch643
WCH_REV  := 62bc06afb6c8a3a8ffe14d05ea4d7066f1dbfee3
WCH      := vendor/ch643/EVT/EXAM
SDK      := $(WCH)/SRC

ARCH    := -march=rv32imac_zicsr -mabi=ilp32
CFLAGS  := $(ARCH) -Os -g -std=gnu11 -Wall -Wextra -ffreestanding \
           -ffunction-sections -fdata-sections -fno-common -msmall-data-limit=8 \
           -Isrc -I$(SDK)/Core -I$(SDK)/Peripheral/inc -I$(WCH)/TOUCHKEY/TKey/User \
           $(CFLAGS_EXTRA)
LDFLAGS := $(ARCH) -nostartfiles -nostdlib -T build/link.ld \
           -Wl,--gc-sections -Wl,-Map=build/hh27.map -Wl,--print-memory-usage

# the name tag: make NAME="ANNE JAN"
ifneq ($(NAME),)
CFLAGS += -DBADGE_NAME='"$(NAME)"'
endif

SRCS := $(wildcard src/*.c src/games/*.c) $(WCH)/TOUCHKEY/TKey/User/system_ch643.c
ASMS := $(SDK)/Startup/startup_ch643.S
OBJS := $(patsubst %,build/%.o,$(notdir $(SRCS) $(ASMS)))

vpath %.c src src/games $(WCH)/TOUCHKEY/TKey/User
vpath %.S $(SDK)/Startup

all: build/hh27.bin size

vendor/ch643:
	git init -q $@
	git -C $@ fetch -q --depth 1 $(WCH_REPO) $(WCH_REV)
	git -C $@ checkout -q FETCH_HEAD

$(SRCS) $(ASMS): | vendor/ch643

build/%.c.o: %.c | build vendor/ch643
	$(CC) $(CFLAGS) -c $< -o $@

build/%.S.o: %.S | build vendor/ch643
	$(CC) $(ARCH) -x assembler-with-cpp -c $< -o $@

# WCH's linker script, with the code flash one 256-byte page shorter: that
# page keeps the name (src/store.c)
build/link.ld: | build vendor/ch643
	sed 's/LENGTH = 62K/LENGTH = 62K - 256/' $(SDK)/Ld/Link.ld > $@

build/hh27.elf: $(OBJS) build/link.ld
	$(CC) $(LDFLAGS) $(OBJS) -lgcc -o $@

build/hh27.bin: build/hh27.elf
	$(OBJCOPY) -O binary $< $@

build:
	mkdir -p build

size: build/hh27.elf
	$(SIZE) $<

flash: build/hh27.bin
	wchisp flash build/hh27.elf

clean:
	rm -rf build

# The hardware-independent part as a shared library for the emulator,
# built with the host compiler (see ../tools/emu/badge_emu.h).
HOSTCC   ?= cc
EMU_SRCS := src/app.c src/anim.c src/console.c src/editor.c src/frame.c src/generated.c src/game.c src/irproto.c src/proto.c \
            $(wildcard src/games/*.c) src/nametag.c emu/emu.c

emu: build/libhh27.so

build/libhh27.so: $(EMU_SRCS) $(wildcard src/*.h) ../tools/emu/badge_emu.h | build
	$(HOSTCC) -std=gnu11 -O2 -g -Wall -Wextra -fPIC -shared -fvisibility=hidden -Isrc \
	    $(if $(NAME),-DBADGE_NAME='"$(NAME)"') $(EMU_SRCS) -o $@

.PHONY: all size flash clean emu
