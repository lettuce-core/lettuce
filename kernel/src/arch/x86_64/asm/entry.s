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

    ljmp 0x08, offset long_mode_start

.code64
long_mode_start:
    mov rsp, offset stack64_top
    xor rbp, rbp
    call setup_syscall_entry
    mov edi, dword ptr [boot_magic_value]
    mov esi, dword ptr [boot_info_ptr_value]
    call rust_main

hang:
    hlt
    jmp hang
