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
    console::init();

    let boot_report = boot::source::BootReport::detect(boot_magic);
    let memory_report = memory::init(boot_info_ptr as usize);
    let syscall_report = syscall::init();

    let boot_info_state = match boot_info_ptr {
        0 => "boot info ptr: missing",
        _ => "boot info ptr: present",
    };
    let boot_info_parse = parse_boot_info_label(boot_info_ptr as usize);

    console::clear_screen(0x1f);

    let mut row = 0usize;
    write_boot_line(&mut row, config::OS_NAME);
    write_boot_line(&mut row, "kernel is working");
    row += 1;

    write_boot_line(&mut row, memory_report.label());
    write_boot_line(&mut row, memory_report.probe_label());
    write_boot_line(&mut row, syscall_report.label());
    write_boot_line(&mut row, "syscall: int80 entry wired");
    row += 1;

    write_boot_line(&mut row, boot_report.source_label());
    write_boot_line(&mut row, boot_report.validation_label());
    write_boot_line(&mut row, boot_info_state);
    write_boot_line(&mut row, boot_info_parse);
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

fn parse_boot_info_label(boot_info_ptr: usize) -> &'static str {
    if boot_info_ptr == 0 {
        return "boot info: not parsed";
    }

    let parsed = unsafe { boot::multiboot2::MultibootInfo::parse(boot_info_ptr) };

    match parsed {
        Ok(info) => info.summary().label(),
        Err(boot::multiboot2::BootInfoError::NullPointer) => "boot info: null pointer",
        Err(boot::multiboot2::BootInfoError::MisalignedPointer) => "boot info: pointer misaligned",
        Err(boot::multiboot2::BootInfoError::InvalidSize) => "boot info: invalid size",
    }
}
