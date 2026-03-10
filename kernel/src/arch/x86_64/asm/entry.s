.section .text
.code32
.global _start
.type _start, @function
_start:
    cli
    mov dword ptr [boot_magic_value], eax
    mov dword ptr [boot_info_ptr_value], ebx
    mov esp, offset stack32_top

    call build_identity_map_2m
    call enable_long_mode

    ljmp KERNEL_CS, offset long_mode_start

.code64
long_mode_start:
    mov rsp, offset stack64_top
    xor rbp, rbp
    mov ax, KERNEL_DS
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    mov edi, dword ptr [boot_magic_value]
    mov esi, dword ptr [boot_info_ptr_value]
    call rust_main

hang:
    hlt
    jmp hang
