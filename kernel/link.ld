OUTPUT_FORMAT("elf64-x86-64")
ENTRY(_start)

/* KERNEL_BASE = 0xFFFFFFFF80000000; */
KERNEL_BASE = 0xFFFFFF8000000000;

SECTIONS {
    . = KERNEL_BASE;

    .text : ALIGN(4K) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
    }

    .got : ALIGN(4K) {
        *(.got .got.*)
    }

    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.eh_frame_hdr)
        /* *(.got) */
        *(.comment)
    }
}
