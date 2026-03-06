.set CR0_PE,       0x00000001
.set CR0_PG,       0x80000000
.set CR4_PAE,      0x00000020
.set EFER_MSR,     0xC0000080
.set EFER_LME,     0x00000100
.set PAGE_PRESENT, 0x1
.set PAGE_WRITE,   0x2
.set PAGE_HUGE,    0x80

.section .bss
.align 16
stack32_bottom:
    .skip 16384
stack32_top:

.align 16
stack64_bottom:
    .skip 16384
stack64_top:

.align 4096
pml4_table:
    .skip 4096
.align 4096
pdpt_table:
    .skip 4096
.align 4096
pd_table:
    .skip 4096

.align 8
boot_magic_value:
    .quad 0

.align 8
boot_info_ptr_value:
    .quad 0

.section .rodata
.align 8
gdt64:
    .quad 0x0000000000000000
    .quad 0x00AF9A000000FFFF
gdt64_end:

gdt64_desc:
    .word gdt64_end - gdt64 - 1
    .quad gdt64

.section .text
.code32

build_identity_map_2m:
    mov eax, offset pdpt_table
    or eax, PAGE_PRESENT | PAGE_WRITE
    mov dword ptr [pml4_table], eax
    mov dword ptr [pml4_table + 4], 0

    mov eax, offset pd_table
    or eax, PAGE_PRESENT | PAGE_WRITE
    mov dword ptr [pdpt_table], eax
    mov dword ptr [pdpt_table + 4], 0

    xor ecx, ecx
build_pd_loop:
    mov eax, ecx
    shl eax, 21
    or eax, PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE
    mov dword ptr [pd_table + ecx * 8], eax
    mov dword ptr [pd_table + ecx * 8 + 4], 0
    inc ecx
    cmp ecx, 512
    jne build_pd_loop

    ret

enable_long_mode:
    lgdt [gdt64_desc]

    mov eax, cr4
    or eax, CR4_PAE
    mov cr4, eax

    mov eax, offset pml4_table
    mov cr3, eax

    mov ecx, EFER_MSR
    rdmsr
    or eax, EFER_LME
    wrmsr

    mov eax, cr0
    or eax, CR0_PE | CR0_PG
    mov cr0, eax

    ret
