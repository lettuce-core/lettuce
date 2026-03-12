pub mod abi;
pub mod dispatch;

pub fn init() {
    core::hint::black_box(&dispatch::REGISTERED_SYSCALLS);
}

// register state pushed by the syscall entry stub (int 0x80)
// 
#[repr(C)]
pub struct SyscallFrame {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

#[no_mangle]
pub extern "C" fn syscall_entry_rust(frame: *mut SyscallFrame) {
    let frame = unsafe { &mut *frame };

    // number in rax, args in rdi/rsi/rdx/r10/r8/r9
    let args = [frame.rdi, frame.rsi, frame.rdx, frame.r10, frame.r8, frame.r9];
    let result = dispatch::dispatch(frame.rax, args);

    frame.rax = result.value;
    frame.rdx = result.error;
}

#[cfg(any(test, feature = "selftest"))]
#[cfg(target_arch = "x86_64")]
pub fn self_test_label() -> &'static str {
    let nr = abi::SyscallNumber::MemAllocFrame as u64;
    let mut value = nr;
    let mut error: u64;

    unsafe {
        core::arch::asm!(
            "int 0x80",
            inout("rax") value,
            in("rdi") 0_u64,
            in("rsi") 0_u64,
            in("rdx") 0_u64,
            in("r10") 0_u64,
            in("r8")  0_u64,
            in("r9")  0_u64,
            lateout("rdx") error,
        );
    }

    if error == 0 && value != 0 {
        "syscall: int80 path ok"
    } else {
        "syscall: int80 path failed"
    }
}

#[cfg(any(test, feature = "selftest"))]
#[cfg(not(target_arch = "x86_64"))]
pub fn self_test_label() -> &'static str {
    "syscall: int80 unsupported on host arch"
}
