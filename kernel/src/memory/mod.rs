pub mod layout;
pub mod pmm;
pub mod vmm;

use layout::{MemoryLayout, MemorySpan, MAX_AVAILABLE_REGIONS, MAX_RESERVED_RANGES};

unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryInitReport {
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
    pub page_size: usize,
    pub pmm_from_mmap: bool,
    pub pmm_probe_ok: bool,
}

impl MemoryInitReport {
    pub fn label(self) -> &'static str {
        if self.pmm_from_mmap {
            "memory: pmm initialized from multiboot2 mmap"
        } else {
            "memory: pmm initialized from early fallback config"
        }
    }

    pub fn probe_label(self) -> &'static str {
        if self.pmm_probe_ok {
            "memory: pmm frame alloc/free probe passed"
        } else {
            "memory: pmm frame alloc/free probe failed"
        }
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

    MemoryInitReport {
        total_frames: pmm_stats.total_frames,
        used_frames: pmm_stats.used_frames,
        free_frames: pmm_stats.free_frames,
        page_size: vmm_report.page_size,
        pmm_from_mmap,
        pmm_probe_ok,
    }
}

fn try_init_pmm_from_boot_layout(boot_info_ptr: usize) -> Result<(), ()> {
    let layout = MemoryLayout::from_boot_info(boot_info_ptr, kernel_image_span())
        .map_err(|_| ())?;

    let mut available_regions = [
        pmm::MemoryRegion {
            start_addr: 0,
            len: 0,
        }; MAX_AVAILABLE_REGIONS
    ];
    
    let mut reserved_ranges = [
        pmm::ReservedRange {
            start_addr: 0,
            len: 0,
        }; MAX_RESERVED_RANGES
    ];

    copy_available_regions(&mut available_regions, layout.available_regions());
    copy_reserved_ranges(&mut reserved_ranges, layout.reserved_ranges());

    pmm::init_from_memory_map(
        pmm::PmmInitInput {
            available_regions: &available_regions[..layout.available_regions().len()],
            reserved_ranges: &reserved_ranges[..layout.reserved_ranges().len()],
        }
    )
    .map_err(|_| ())
}

fn kernel_image_span() -> MemorySpan {
    let start_addr = unsafe { (&__kernel_start as *const u8) as usize };
    let end_addr = unsafe { (&__kernel_end as *const u8) as usize };

    MemorySpan::new(start_addr, end_addr.saturating_sub(start_addr))
}

fn copy_available_regions(dst: &mut [pmm::MemoryRegion], src: &[MemorySpan]) {
    for (dst, src) in dst.iter_mut()
        .zip(src.iter().copied()) 
    {
        *dst = pmm::MemoryRegion {
            start_addr: src.start_addr,
            len: src.len,
        };
    }
}

fn copy_reserved_ranges(dst: &mut [pmm::ReservedRange], src: &[MemorySpan]) {
    for (dst, src) in dst.iter_mut()
        .zip(src.iter().copied()) 
    {
        *dst = pmm::ReservedRange {
            start_addr: src.start_addr,
            len: src.len,
        };
    }
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
