#!/bin/sh
# check_size.sh ELF LIMIT_BYTES [NAME]
# Prints the flash and RAM a firmware ELF uses and fails when the flash
# (text + data: code, constants and the initial values of variables) is
# over LIMIT_BYTES. The code flash is 62 KiB - 256 = 63232 bytes (the last
# page keeps the name, see memory.x / c/Makefile).
# Uses $SIZE (default: size; GNU size reads RISC-V ELFs on any host).
set -eu
elf=$1
limit=$2
name=${3:-$elf}
set -- $(${SIZE:-size} "$elf" | awk 'NR == 2 { print $1, $2, $3 }')
text=$1 data=$2 bss=$3
flash=$((text + data))
ram=$((data + bss))
region=63232
pct=$((flash * 1000 / region))
printf '%s: flash %d bytes (%d.%d%% of %d), static RAM %d bytes; limit %d\n' \
    "$name" "$flash" $((pct / 10)) $((pct % 10)) "$region" "$ram" "$limit"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    printf '| %s | %d | %d.%d%% | %d | %d |\n' "$name" "$flash" $((pct / 10)) $((pct % 10)) "$ram" "$limit" >> "$GITHUB_STEP_SUMMARY"
fi
if [ "$flash" -gt "$limit" ]; then
    echo "::error::$name uses $flash bytes of flash, over the limit of $limit"
    exit 1
fi
