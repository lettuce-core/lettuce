pub mod heap;
pub mod layout;
pub mod pmm;
pub mod vmm;

use layout::{MemoryLayout, MemorySpan};
use core::alloc::Layout;

unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryInitReport {
    pub tracked_frames: usize,
    pub usable_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
    pub heap_capacity_bytes: usize,
    pub heap_used_bytes: usize,
    pub heap_free_bytes: usize,
    pub page_size: usize,
    pub pmm_from_mmap: bool,
    pub pmm_probe_ok: bool,
    pub heap_probe_ok: bool,
}

impl MemoryInitReport {
    pub fn label(self) -> &'static str {
        if self.pmm_from_mmap {
            "memory: pmm initialized from multiboot2 mmap"
        } else {
            "memory: pmm initialized from early fallback config"
        }
    }
    
    pub fn frames_summary_line<'a>(self, buf: &'a mut [u8; 96]) -> &'a str {
        let mut line = FixedLineBuf::new(buf);

        line.push_str("memory frames: tracked ");
        line.push_usize(self.tracked_frames);
        line.push_str(" usable ");
        line.push_usize(self.usable_frames);
        line.push_str(" used ");
        line.push_usize(self.used_frames);
        line.push_str(" free ");
        line.push_usize(self.free_frames);

        line.into_str()
    }

    pub fn heap_summary_line<'a>(self, buf: &'a mut [u8; 80]) -> &'a str {
        let mut line = FixedLineBuf::new(buf);

        line.push_str("kernel heap: capacity ");
        line.push_usize(self.heap_capacity_bytes);
        line.push_str(" used ");
        line.push_usize(self.heap_used_bytes);
        line.push_str(" free ");
        line.push_usize(self.heap_free_bytes);

        line.into_str()
    }
}

pub fn init(boot_info_ptr: usize) -> MemoryInitReport {
    let pmm_from_mmap = try_init_pmm_from_boot_layout(boot_info_ptr).is_ok();

    if !pmm_from_mmap {
        pmm::init(pmm::EarlyPmmConfig::default());
    }

    let pmm_probe_ok = pmm_probe();
    let pmm_stats = pmm::stats().expect("pmm must be initialized");
    let vmm_report = vmm::init();
    heap::init().expect("early heap must initialize");
    let heap_probe_ok = heap_probe();
    let heap_stats = heap::stats().expect("heap must be initialized");

    MemoryInitReport {
        tracked_frames: pmm_stats.tracked_frames,
        usable_frames: pmm_stats.usable_frames,
        used_frames: pmm_stats.used_frames,
        free_frames: pmm_stats.free_frames,
        heap_capacity_bytes: heap_stats.capacity_bytes,
        heap_used_bytes: heap_stats.used_bytes,
        heap_free_bytes: heap_stats.free_bytes,
        page_size: vmm_report.page_size,
        pmm_from_mmap,
        pmm_probe_ok,
        heap_probe_ok,
    }
}

fn try_init_pmm_from_boot_layout(boot_info_ptr: usize) -> Result<(), ()> {
    let layout = MemoryLayout::from_boot_info(boot_info_ptr, kernel_image_span())
        .map_err(|_| ())?;

    pmm::init_from_layout(&layout).map_err(|_| ())
}

fn kernel_image_span() -> MemorySpan {
    let start_addr = unsafe { (&__kernel_start as *const u8) as usize };
    let end_addr = unsafe { (&__kernel_end as *const u8) as usize };

    MemorySpan::new(start_addr, end_addr.saturating_sub(start_addr))
}

fn pmm_probe() -> bool {
    let frame_a = match pmm::alloc_frame() {
        Ok(frame) => frame,
        Err(_) => return false,
    };

    let frame_b = match pmm::alloc_frame() {
        Ok(frame) => frame,
        Err(_) => {
            let _ = pmm::free_frame(frame_a);
            return false;
        }
    };

    pmm::free_frame(frame_b).is_ok() && pmm::free_frame(frame_a).is_ok()
}

fn heap_probe() -> bool {
    let layout_a = match Layout::from_size_align(32, 8) {
        Ok(layout) => layout,
        Err(_) => return false,
    };
    let layout_b = match Layout::from_size_align(64, 16) {
        Ok(layout) => layout,
        Err(_) => return false,
    };

    let block_a = match heap::alloc(layout_a) {
        Ok(ptr) => ptr,
        Err(_) => return false,
    };
    let block_b = match heap::alloc_zeroed(layout_b) {
        Ok(ptr) => ptr,
        Err(_) => return false,
    };

    if block_a.as_ptr() as usize % layout_a.align() != 0 {
        return false;
    }

    if block_b.as_ptr() as usize % layout_b.align() != 0 {
        return false;
    }

    for byte in 0..layout_b.size() {
        let value = unsafe { block_b.as_ptr().add(byte).read() };
        if value != 0 {
            return false;
        }
    }

    true
}

struct FixedLineBuf<'a> {
    bytes: &'a mut [u8],
    len: usize,
}

impl<'a> FixedLineBuf<'a> {
    fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, len: 0 }
    }

    fn push_str(&mut self, value: &str) {
        for byte in value.bytes() {
            if self.len >= self.bytes.len() {
                return;
            }

            self.bytes[self.len] = byte;
            self.len += 1;
        }
    }

    fn push_usize(&mut self, value: usize) {
        let mut digits = [0u8; 20];
        let mut cursor = digits.len();
        let mut value = value;

        if value == 0 {
            self.push_byte(b'0');
            return;
        }

        while value > 0 && cursor > 0 {
            cursor -= 1;
            digits[cursor] = b'0' + (value % 10) as u8;
            value /= 10;
        }

        for byte in &digits[cursor..] {
            self.push_byte(*byte);
        }
    }

    fn push_byte(&mut self, byte: u8) {
        if self.len >= self.bytes.len() {
            return;
        }

        self.bytes[self.len] = byte;
        self.len += 1;
    }

    fn into_str(self) -> &'a str {
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.len]) }
    }
}
