pub mod pmm;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryInitReport {
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
}

impl MemoryInitReport {
    pub fn label(self) -> &'static str {
        "memory: pmm initialized"
    }
}

pub fn init() -> MemoryInitReport {
    pmm::init(pmm::EarlyPmmConfig::default());

    let stats = pmm::stats().expect("pmm must be initialized");
    MemoryInitReport {
        total_frames: stats.total_frames,
        used_frames: stats.used_frames,
        free_frames: stats.free_frames,
    }
}
