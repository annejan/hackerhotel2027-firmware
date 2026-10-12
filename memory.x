/* CH643 memory, as ch32-metapac's, except that the code flash ends one
 * 256-byte page early: that page (0x0800F700 / 0x0000F700) keeps the
 * name, see src/store.rs. */
MEMORY
{
    CODE   (rx)  : ORIGIN = 0x00000000, LENGTH =   62K - 256 /* USR boot alias */
    USR    (rwx) : ORIGIN = 0x08000000, LENGTH =   62K
    SYS    (rwx) : ORIGIN = 0x1FFF0000, LENGTH =  3328
    VND    (r)   : ORIGIN = 0x1FFFF700, LENGTH =   256
    OPT    (rw)  : ORIGIN = 0x1FFFF800, LENGTH =   256
    RAM    (rwx) : ORIGIN = 0x20000000, LENGTH =   20K
}

REGION_ALIAS("FLASH", CODE);

REGION_ALIAS("REGION_TEXT", CODE);
REGION_ALIAS("REGION_RODATA", CODE);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);
