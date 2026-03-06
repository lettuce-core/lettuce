use core::ptr::write_volatile;

const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const VGA_BUFFER_ADDR: usize = 0xb8000;
const DEFAULT_COLOR: u8 = 0x1f;

pub fn clear_screen(color: u8) {
    let mut ptr = VGA_BUFFER_ADDR as *mut u8;

    for _ in 0..(VGA_WIDTH * VGA_HEIGHT) {
        unsafe {
            write_volatile(ptr, b' ');
            write_volatile(ptr.add(1), color);
            ptr = ptr.add(2);
        }
    }
}

pub fn write_line(row: usize, text: &str) {
    if row >= VGA_HEIGHT {
        return;
    }

    let base = VGA_BUFFER_ADDR + (row * VGA_WIDTH * 2);
    let mut ptr = base as *mut u8;

    for byte in text.bytes().take(VGA_WIDTH) {
        unsafe {
            write_volatile(ptr, byte);
            write_volatile(ptr.add(1), DEFAULT_COLOR);
            ptr = ptr.add(2);
        }
    }
}
