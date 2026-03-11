.set CR0_PE, 0x00000001
.set CR0_PG, 0x80000000
.set CR4_PAE, 0x00000020
.set EFER_MSR, 0xC0000080
.set EFER_LME, 0x00000100
.set PAGE_PRESENT, 0x1
.set PAGE_WRITE, 0x2
.set PAGE_HUGE, 0x80

.set KERNEL_CS, 0x08
.set KERNEL_DS, 0x10

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
    .quad 0x00AF92000000FFFF
gdt64_end:

gdt64_desc:
    .word gdt64_end - gdt64 - 1
    .quad gdt64

.section .bss
.align 16
idt64_table:
    .skip 4096

.section .data
idt64_desc:
    .word 4096 - 1
    .quad idt64_table

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
1:
    mov eax, ecx
    shl eax, 21
    or eax, PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE
    mov dword ptr [pd_table + ecx * 8], eax
    mov dword ptr [pd_table + ecx * 8 + 4], 0
    inc ecx
    cmp ecx, 512
    jne 1b

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

.code64
.global idt_set_gate
idt_set_gate:
    movzx eax, dil
    shl rax, 4
    lea rcx, [rip + idt64_table]
    add rcx, rax

    mov rax, rsi
    mov word ptr [rcx + 0], ax
    mov word ptr [rcx + 2], KERNEL_CS
    mov byte ptr [rcx + 4], 0
    mov byte ptr [rcx + 5], dl
    shr rax, 16
    mov word ptr [rcx + 6], ax
    shr rax, 16
    mov dword ptr [rcx + 8], eax
    mov dword ptr [rcx + 12], 0
    ret

.global idt_load
idt_load:
    lidt [idt64_desc]
    ret

.macro enter_exception_common vector:req, has_error:req, read_cr2:req
    mov r8, rsp
    mov rdi, \vector

    .if \has_error
        mov rsi, [r8 + 0]
        mov rdx, [r8 + 8]
    .else
        xor rsi, rsi
        mov rdx, [r8 + 0]
    .endif

    .if \read_cr2
        mov rcx, cr2
    .else
        xor rcx, rcx
    .endif

    and rsp, -16
    sub rsp, 8
    call exception_entry_rust
1:
    cli
    hlt
    jmp 1b
.endm

.global exc_divide_error_stub
exc_divide_error_stub:
    enter_exception_common 0, 0, 0

.global exc_invalid_opcode_stub
exc_invalid_opcode_stub:
    enter_exception_common 6, 0, 0

.global exc_double_fault_stub
exc_double_fault_stub:
    enter_exception_common 8, 1, 0

.global exc_general_protection_stub
exc_general_protection_stub:
    enter_exception_common 13, 1, 0

.global exc_page_fault_stub
exc_page_fault_stub:
    enter_exception_common 14, 1, 1
