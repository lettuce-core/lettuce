use super::{serial, vga};

// initialize all active output backends for early kernel logs
pub fn init() {
    serial::init();
}

// clears vga screen to a given color
pub fn clear_screen(color: u8) {
    vga::clear_screen(color);
}

// unified write path so one call updates serial and vga together
pub fn write_line(row: usize, msg: &str) {
    serial::write_line(msg);
    vga::write_line(row, msg);
}
