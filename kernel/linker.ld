ENTRY(_start)

SECTIONS
{
  . = 1M;
  __kernel_start = .;

  .multiboot_header :
  {
    KEEP(*(.multiboot_header))
  }

  .text :
  {
    *(.text .text.*)
  }

  .rodata :
  {
    *(.rodata .rodata.*)
  }

  .data :
  {
    *(.data .data.*)
  }

  .bss :
  {
    *(.bss .bss.* COMMON)
  }

  __kernel_end = .;
}
