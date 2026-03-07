#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod boot;
mod config;
mod memory;

mod console;
mod serial;
mod syscall;
mod vga;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/arch/x86_64/asm/core.s"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/arch/x86_64/asm/entry.s"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/arch/x86_64/asm/multiboot.s"
    )),
);

#[no_mangle]
pub extern "C" fn rust_main(boot_magic: u32, boot_info_ptr: u32) -> ! {
    let boot_report = boot::source::BootReport::detect(boot_magic);
    let memory_report = memory::init();
    let syscall_report = syscall::init();
    let boot_info_state = if boot_info_ptr == 0 {
        "boot info ptr: missing"
    } else {
        "boot info ptr: present"
    };

    console::init();
    console::clear_screen(0x1f);

    let mut row = 0usize;
    write_boot_line(&mut row, config::OS_NAME);
    write_boot_line(&mut row, "kernel is working");
    write_boot_line(&mut row, memory_report.label());
    write_boot_line(&mut row, syscall_report.label());
    write_boot_line(&mut row, "syscall: int80 entry wired");
    row += 1;
    write_boot_line(&mut row, boot_report.source_label());
    write_boot_line(&mut row, boot_report.validation_label());
    write_boot_line(&mut row, boot_info_state);
    row += 1;
    write_boot_line(&mut row, "boot path reached rust_main()");

    halt_forever()
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    console::clear_screen(0x4f);
    console::write_line(0, "kernel panic");
    halt_forever()
}

fn halt_forever() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }

        #[cfg(not(target_arch = "x86_64"))]
        core::hint::spin_loop();
    }
}

fn write_boot_line(row: &mut usize, msg: &str) {
    console::write_line(*row, msg);
    *row += 1;
}
