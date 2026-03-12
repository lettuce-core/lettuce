pub mod config;
pub mod console;
pub mod fmtbuf;
pub mod serial;
pub mod vga;

// rounds `value` up to the nearest multiple of `align`
// returns `None` on overflow. `align` must be a power of two
// 
pub fn align_up(value: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());
    value.checked_add(align - 1).map(|v| v & !(align - 1))
}

// rounds `value` down to the nearest multiple of `align`
// `align` must be a power of two
// 
pub fn align_down(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    value & !(align - 1)
}
