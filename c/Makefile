# C port of the HH2027 starfish firmware, for size comparison with the Rust one.
#
#   make            build build/hh27.elf / .bin (fetches the WCH SDK on first run)
#   make size       section sizes
#   make flash      flash with wlink (WCH-LinkE)
#
# Uses only startup, linker script, clock init and register headers from
# WCH's SDK (github.com/openwch/ch643); no peripheral library, no libc.
# The SDK is fetched at build time and not committed.

CROSS   ?= riscv64-elf-
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
           -Isrc -I$(SDK)/Core -I$(SDK)/Peripheral/inc -I$(WCH)/TOUCHKEY/TKey/User
LDFLAGS := $(ARCH) -nostartfiles -nostdlib -T $(SDK)/Ld/Link.ld \
           -Wl,--gc-sections -Wl,-Map=build/hh27.map -Wl,--print-memory-usage

SRCS := $(wildcard src/*.c) $(WCH)/TOUCHKEY/TKey/User/system_ch643.c
ASMS := $(SDK)/Startup/startup_ch643.S
OBJS := $(patsubst %,build/%.o,$(notdir $(SRCS) $(ASMS)))

vpath %.c src $(WCH)/TOUCHKEY/TKey/User
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

build/hh27.elf: $(OBJS)
	$(CC) $(LDFLAGS) $^ -o $@

build/hh27.bin: build/hh27.elf
	$(OBJCOPY) -O binary $< $@

build:
	mkdir -p build

size: build/hh27.elf
	$(SIZE) $<

flash: build/hh27.bin
	wlink flash build/hh27.elf

clean:
	rm -rf build

.PHONY: all size flash clean
