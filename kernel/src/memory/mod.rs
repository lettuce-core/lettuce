pub mod heap;
pub mod layout;
pub mod pmm;
pub mod vmm;

use crate::utils::fmtbuf::FixedBuf;
use core::fmt::Write;

use layout::{MemoryLayout, MemorySpan};
use core::alloc::Layout;

const PROBE_LAYOUT_A: Layout = unsafe { Layout::from_size_align_unchecked(32, 8) };
const PROBE_LAYOUT_B: Layout = unsafe { Layout::from_size_align_unchecked(64, 16) };

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
    pub kernel_root_table: usize,
    pub identity_map_bytes: usize,
    
    pub pmm_from_mmap: bool,
    pub pmm_probe_ok: bool,
    pub heap_probe_ok: bool,
    pub vmm_probe_ok: bool,
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
        fmt_line(
            buf, format_args!(
                "memory frames: tracked {} usable {} used {} free {}",
                self.tracked_frames, 
                self.usable_frames, 
                self.used_frames, 
                self.free_frames,
            )
        )
    }

    pub fn heap_summary_line<'a>(self, buf: &'a mut [u8; 80]) -> &'a str {
        fmt_line(
            buf, format_args!(
                "kernel heap: capacity {} used {} free {}",
                self.heap_capacity_bytes,
                self.heap_used_bytes,
                self.heap_free_bytes,
            )
        )
    }

    pub fn vmm_summary_line<'a>(self, buf: &'a mut [u8; 96]) -> &'a str {
        fmt_line(
            buf, format_args!(
                "vmm: root {:#x} identity {} bytes",
                self.kernel_root_table,
                self.identity_map_bytes,
            )
        )
    }

    pub fn vmm_probe_label(self) -> &'static str {
        if self.vmm_probe_ok {
            "vmm: active kernel address space verified"
        } else {
            "vmm: kernel address space probe failed"
        }
    }
}

pub fn init(boot_info_ptr: usize) -> MemoryInitReport {
    let pmm_from_mmap = try_init_pmm_from_boot_layout(boot_info_ptr).is_ok();

    if !pmm_from_mmap {
        let _ = pmm::init(pmm::EarlyPmmConfig::default());
    }

    let pmm_probe_ok = pmm_probe();
    let pmm_stats = pmm::stats().expect("pmm must be initialized");
    let vmm_report = vmm::init();
    
    let vmm_probe_ok = vmm_probe(boot_info_ptr, vmm_report);
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
        kernel_root_table: vmm_report.kernel_root_table.0,
        identity_map_bytes: vmm_report.identity_map_bytes,
        
        pmm_from_mmap,
        pmm_probe_ok,
        heap_probe_ok,
        vmm_probe_ok,
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

    // free in reverse allocation order
    pmm::free_frame(frame_b).is_ok() && pmm::free_frame(frame_a).is_ok()
}

fn heap_probe() -> bool {
    let block_a = match heap::alloc(PROBE_LAYOUT_A) {
        Ok(ptr) => ptr,
        Err(_) => return false,
    };
    
    let block_b = match heap::alloc_zeroed(PROBE_LAYOUT_B) {
        Ok(ptr) => ptr,
        Err(_) => return false,
    };

    if !is_aligned(block_a.as_ptr(), PROBE_LAYOUT_A.align()) { return false; }
    if !is_aligned(block_b.as_ptr(), PROBE_LAYOUT_B.align()) { return false; }

    let slice = unsafe {
        core::slice::from_raw_parts(block_b.as_ptr(), PROBE_LAYOUT_B.size())
    };
    
    if slice.iter().any(|&b| b != 0) { return false; }
    true
}

// :: todo: verify using a real kernel VA (currently checks only boot identity map)
fn vmm_probe(boot_info_ptr: usize, vmm_report: vmm::VmmInitReport) -> bool {
    let s = match vmm::kernel_address_space() {
        Ok(space) => space,
        Err(_) => return false,
    };

    if s.root_table() != vmm_report.kernel_root_table { return false; }
    if s.identity_map_bytes() != vmm_report.identity_map_bytes { return false; }

    if boot_info_ptr == 0 {
        return true;
    }

    match vmm::identity_map_addr(vmm::PhysAddr(boot_info_ptr)) {
        Ok(va) => va.0 == boot_info_ptr,
        Err(_) => false,
    }
}

fn is_aligned(ptr: *mut u8, align: usize) -> bool {
    ptr as usize % align == 0
}

fn fmt_line<'a>(buf: &'a mut [u8], args: core::fmt::Arguments<'_>) -> &'a str {
    let mut b = FixedBuf::new(buf);
    b.write_fmt(args).ok();
    b.into_str()
}
