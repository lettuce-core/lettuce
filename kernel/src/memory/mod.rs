pub mod pmm;
pub mod vmm;

use crate::boot::multiboot2::MultibootInfo;

const MAX_MMAP_REGIONS: usize = 64;
const MAX_RESERVED_RANGES: usize = 4;
const LOW_MEMORY_RESERVE: usize = 0x10_0000;

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
    let pmm_from_mmap = try_init_pmm_from_multiboot2(boot_info_ptr).is_ok();

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

fn try_init_pmm_from_multiboot2(boot_info_ptr: usize) -> Result<(), ()> {
    let info = unsafe { MultibootInfo::parse(boot_info_ptr) }.map_err(|_| ())?;

    let (_, mmap_iter) = info.memory_map().ok_or(())?;

    let mut regions = [pmm::MemoryRegion {
        start_addr: 0,
        len: 0,
    }; MAX_MMAP_REGIONS];

    let mut region_count = 0usize;

    for entry in mmap_iter {
        if !entry.is_available() {
            continue;
        }

        if region_count >= MAX_MMAP_REGIONS {
            break;
        }

        regions[region_count] = pmm::MemoryRegion {
            start_addr: entry.base_addr as usize,
            len: entry.length as usize,
        };
        region_count += 1;
    }

    if region_count == 0 {
        return Err(());
    }

    let kernel_start = unsafe { (&__kernel_start as *const u8) as usize };
    let kernel_end = unsafe { (&__kernel_end as *const u8) as usize };

    let mut reserved = [pmm::ReservedRange {
        start_addr: 0,
        len: 0,
    }; MAX_RESERVED_RANGES];

    reserved[0] = pmm::ReservedRange {
        start_addr: 0,
        len: LOW_MEMORY_RESERVE,
    };

    reserved[1] = pmm::ReservedRange {
        start_addr: kernel_start,
        len: kernel_end.saturating_sub(kernel_start),
    };

    reserved[2] = pmm::ReservedRange {
        start_addr: boot_info_ptr,
        len: info.total_size(),
    };

    pmm::init_from_memory_map(pmm::PmmInitInput {
        available_regions: &regions[..region_count],
        reserved_ranges: &reserved[..3],
    })
    .map_err(|_| ())
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
