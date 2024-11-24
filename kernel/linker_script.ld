OUTPUT_FORMAT("elf64-x86-64")
ENTRY(_start)
SECTIONS{
    .ap_startup = 0x8000;
    . = 0xFFFFFFFF80000000;
    .text ALIGN (0x1000) : {
        *(.text)
        *(.text.*)
        *(.rodata)
        *(.rodata.*)
    }
    .dynamic ALIGN (4096) : {
        *(.dynamic)
    }
    .data ALIGN (4096) : {
        *(.data)
        *(.data.*)
    }
    .bss ALIGN (0x1000) : {
        *(.bss)
        *(.bss.*)
    }
}

