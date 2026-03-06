#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod boot;
mod config;
mod console;
mod serial;
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
pub extern "C" fn rust_main(boot_magic: u32) -> ! {
    let boot_report = boot::source::BootReport::detect(boot_magic);

    console::init();
    console::clear_screen(0x1f);
    console::write_line(0, config::OS_NAME);
    console::write_line(1, "kernel is working");
    console::write_line(3, boot_report.source_label());
    console::write_line(4, boot_report.validation_label());
    console::write_line(6, "boot path reached rust_main()");

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
