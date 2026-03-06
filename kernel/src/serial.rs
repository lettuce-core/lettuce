#[cfg(target_arch = "x86_64")]
const COM1_PORT: u16 = 0x3f8;

pub fn init() {
    init_impl();
}

pub fn write_line(msg: &str) {
    write_line_impl(msg);
}

#[cfg(target_arch = "x86_64")]
fn init_impl() {
    write_port(COM1_PORT + 1, 0x00);
    write_port(COM1_PORT + 3, 0x80);
    write_port(COM1_PORT + 0, 0x03);
    write_port(COM1_PORT + 1, 0x00);
    write_port(COM1_PORT + 3, 0x03);
    write_port(COM1_PORT + 2, 0xC7);
    write_port(COM1_PORT + 4, 0x0B);
}

#[cfg(not(target_arch = "x86_64"))]
fn init_impl() {}

#[cfg(target_arch = "x86_64")]
fn write_line_impl(msg: &str) {
    for byte in msg.bytes() {
        write_byte(byte);
    }

    write_byte(b'\r');
    write_byte(b'\n');
}

#[cfg(not(target_arch = "x86_64"))]
fn write_line_impl(_msg: &str) {}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn write_byte(byte: u8) {
    while (read_port(COM1_PORT + 5) & 0x20) == 0 {
        core::hint::spin_loop();
    }
    write_port(COM1_PORT, byte);
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn write_port(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn read_port(port: u16) -> u8 {
    let mut value: u8;

    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}
