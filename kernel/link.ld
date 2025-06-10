OUTPUT_FORMAT("elf64-x86-64")
ENTRY(_start)

KERNEL_BASE = 0xFFFFFFFF80000000;

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

    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
