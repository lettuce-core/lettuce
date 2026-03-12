use crate::utils::console;
use crate::utils::fmtbuf::FixedBuf;
use core::fmt::Write;

const GATE_INTERRUPT_DPL0: u8 = 0x8E;

const EXCEPTION_DIVIDE_ERROR: u8 = 0;
const EXCEPTION_INVALID_OPCODE: u8 = 6;
const EXCEPTION_DOUBLE_FAULT: u8 = 8;
const EXCEPTION_GENERAL_PROTECTION: u8 = 13;
const EXCEPTION_PAGE_FAULT: u8 = 14;

unsafe extern "C" {
    fn idt_set_gate(vector: u8, handler: u64, type_attr: u8);
    fn idt_load();

    fn exc_divide_error_stub();
    fn exc_invalid_opcode_stub();
    fn exc_double_fault_stub();
    fn exc_general_protection_stub();
    fn exc_page_fault_stub();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuInitReport;

impl CpuInitReport {
    pub fn label(self) -> &'static str {
        "cpu: idt loaded for core exceptions"
    }
}

pub fn init() -> CpuInitReport {
    const GATES: &[(u8, unsafe extern "C" fn())] = &[
        (EXCEPTION_DIVIDE_ERROR,       exc_divide_error_stub),
        (EXCEPTION_INVALID_OPCODE,     exc_invalid_opcode_stub),
        (EXCEPTION_DOUBLE_FAULT,       exc_double_fault_stub),
        (EXCEPTION_GENERAL_PROTECTION, exc_general_protection_stub),
        (EXCEPTION_PAGE_FAULT,         exc_page_fault_stub),
    ];

    for &(vector, stub) in GATES {
        install_interrupt_gate(vector, stub);
    }

    unsafe {
        idt_load();
    }

    CpuInitReport
}

pub fn install_interrupt_gate(vector: u8, handler: unsafe extern "C" fn()) {
    unsafe {
        idt_set_gate(
            vector, 
            handler_addr(handler), 
            GATE_INTERRUPT_DPL0
        )
    }
}

fn display_exception(vector: u64, error_code: u64, rip: u64, cr2: u64) {
    let mut buf = [0u8; 96];

    console::clear_screen(0x4f);
    console::write_line(0, "cpu exception");
    console::write_line(1, exception_label(vector));

    let mut b = FixedBuf::new(&mut buf);
    write!(
        b, 
        "rip {:#018x}  err {:#018x}", 
        rip, 
        error_code
    )
    .ok();
    console::write_line(2, b.as_str());

    if vector == EXCEPTION_PAGE_FAULT as u64 {
        let mut buf2 = [0u8; 48];
        let mut b2 = FixedBuf::new(&mut buf2);
        
        write!(
            b2, 
            "cr2 {:#018x}", 
            cr2
        )
        .ok();
        console::write_line(3, b2.as_str());
    }
}

#[no_mangle]
pub extern "C" fn exception_entry_rust(vector: u64, error_code: u64, rip: u64, cr2: u64) -> ! {
    display_exception(vector, error_code, rip, cr2);
    halt_forever()
}

fn handler_addr(f: unsafe extern "C" fn()) -> u64 {
    f as usize as u64
}

fn exception_label(vector: u64) -> &'static str {
    match vector as u8 {
        EXCEPTION_DIVIDE_ERROR => "fault: divide error",
        EXCEPTION_INVALID_OPCODE => "fault: invalid opcode",
        EXCEPTION_DOUBLE_FAULT => "fault: double fault",
        EXCEPTION_GENERAL_PROTECTION => "fault: general protection",
        EXCEPTION_PAGE_FAULT => "fault: page fault",
        _ => "fault: unknown exception",
    }
}

fn halt_forever() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("cli", "hlt", options(nomem, nostack));
        }

        #[cfg(not(target_arch = "x86_64"))]
        core::hint::spin_loop();
    }
}
