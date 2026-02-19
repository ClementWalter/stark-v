OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY {
    TEXT (rx)  : ORIGIN = 0x00000400, LENGTH = 0x000FFC00
    DATA (rw)  : ORIGIN = 0x00200000, LENGTH = 0x02000000
}

/* Why: Rust STF execution uses deep call stacks and large stack frames
   (state transition + trie + transaction decoding). A tiny stack causes
   silent corruption and non-halting RV32 runs in the runner. */
__stack_size = 0x00400000; /* 4 MiB */
__stack_top = 0x02200000;
__stack_bottom = __stack_top - __stack_size;

/* I/O region between TEXT and stack. */
__io_start = 0x00100000;
/* Why: block-level STF inputs can be several KiB; reserve enough headroom for
   calldata-heavy fixtures while keeping output room large. */
__input_len = 0x00020000; /* 128 KiB input buffer */
__input_start = __io_start;
__input_end = __input_start + __input_len;
__halt_flag = __input_end;
__output_len = __halt_flag + 4;
__output_data = __halt_flag + 8;
__output_end = __stack_bottom;

SECTIONS {
    .text : ALIGN(4) {
        __text_start = .;
        KEEP(*(.text._start))
        *(.text .text.*)
        *(.gnu.linkonce.t.*)
        __text_end = .;
    } > TEXT

    .rodata : ALIGN(4) {
        __data_start = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        *(.eh_frame*)
    } > DATA

    .bss (NOLOAD) : ALIGN(4) {
        __bss_start = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > DATA

    .data : ALIGN(4) {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } > DATA

    __text_len = __text_end - __text_start;
    __data_end = .;
    __data_len = __data_end - __data_start;

    PROVIDE(__global_pointer$ = ORIGIN(DATA) + 0x800);

    /DISCARD/ : {
        *(.comment)
        *(.note.GNU-stack)
    }
}
