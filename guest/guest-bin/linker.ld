OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY {
    TEXT (rx)  : ORIGIN = 0x00000400, LENGTH = 0x000FFC00
    RODATA (r) : ORIGIN = 0x00100000, LENGTH = 0x00100000
    DATA  (rw) : ORIGIN = 0x00200000, LENGTH = 0x000FFC00
}

__stack_size = 0x00000400; /* 1 KiB */
__stack_top = 0x00300000; /* 0x00100000 + 2 MiB */
__stack_bottom = __stack_top - __stack_size;

SECTIONS {
    .text : ALIGN(4) {
        KEEP(*(.text._start))
        *(.text .text.*)
        *(.gnu.linkonce.t.*)
    } > TEXT

    .rodata : ALIGN(4) {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        *(.eh_frame*)
    } > RODATA

    .data : ALIGN(4) {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } > DATA

    .bss (NOLOAD) : ALIGN(4) {
        __bss_start = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > DATA

    PROVIDE(__global_pointer$ = ORIGIN(DATA) + 0x800);

    /DISCARD/ : {
        *(.comment)
        *(.note.GNU-stack)
    }
}
