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

.code64
setup_syscall_entry:
    lea rax, [rip + syscall_int80_handler]

    // offset low (bits 0..15)
    mov word ptr [idt64_table + 128 * 16 + 0], ax
    // selector (kernel code)
    mov word ptr [idt64_table + 128 * 16 + 2], 0x08
    // ist
    mov byte ptr [idt64_table + 128 * 16 + 4], 0
    // type attr: present + dpl3 + interrupt gate
    mov byte ptr [idt64_table + 128 * 16 + 5], 0xEE

    shr rax, 16
    // offset mid (bits 16..31)
    mov word ptr [idt64_table + 128 * 16 + 6], ax
    shr rax, 16
    // offset high (bits 32..63)
    mov dword ptr [idt64_table + 128 * 16 + 8], eax
    // reserved
    mov dword ptr [idt64_table + 128 * 16 + 12], 0

    lidt [idt64_desc]
    ret

syscall_int80_handler:
    // frame layout at rsp top (repr C):
    // rax, rbx, rcx, rdx, rsi, rdi, r8, r9, r10, r11, r12, r13, r14, r15
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax

    mov rdi, rsp
    call syscall_entry_rust

    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15

    iretq
