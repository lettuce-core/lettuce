#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod boot;
mod config;
mod cpu;
mod memory;
mod fmtbuf;

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

    let cpu_report = cpu::init();
    let boot_report = boot::source::BootReport::detect(boot_magic);
    let memory_report = memory::init(boot_info_ptr as usize);
    syscall::init();

    let boot_info_parse = parse_boot_info_label(boot_info_ptr as usize);
    
    let mut memory_summary = [0u8; 96];
    let mut heap_summary = [0u8; 80];
    let mut vmm_summary = [0u8; 96];

    console::clear_screen(0x1f);

    let mut row = 0usize;
    write_boot_line(&mut row, config::OS_NAME);
    write_boot_line(&mut row, cpu_report.label());
    row += 1;
    
    write_boot_line(&mut row, memory_report.label());
    write_boot_line(&mut row, memory_report.frames_summary_line(&mut memory_summary));
    write_boot_line(&mut row, memory_report.heap_summary_line(&mut heap_summary));
    write_boot_line(&mut row, memory_report.vmm_summary_line(&mut vmm_summary));

    if !memory_report.vmm_probe_ok {
        write_boot_line(&mut row, memory_report.vmm_probe_label());
    }

    row += 1;
    
    write_boot_line(&mut row, boot_report.source_label());
    write_boot_line(&mut row, boot_info_parse);

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
