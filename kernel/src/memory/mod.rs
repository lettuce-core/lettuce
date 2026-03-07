pub mod pmm;
pub mod vmm;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryInitReport {
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
    pub page_size: usize,
}

impl MemoryInitReport {
    pub fn label(self) -> &'static str {
        "memory: pmm + vmm initialized"
    }
}

pub fn init() -> MemoryInitReport {
    pmm::init(pmm::EarlyPmmConfig::default());

    let pmm_stats = pmm::stats().expect("pmm must be initialized");
    let vmm_report = vmm::init();

    MemoryInitReport {
        total_frames: pmm_stats.total_frames,
        used_frames: pmm_stats.used_frames,
        free_frames: pmm_stats.free_frames,
        page_size: vmm_report.page_size,
    }
}
