use crate::boot::multiboot2::{BootInfoError, MultibootInfo};

pub const MAX_AVAILABLE_REGIONS: usize = 64;
pub const MAX_RESERVED_RANGES: usize = 8;
pub const LOW_MEMORY_RESERVE: usize = 0x10_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemorySpan {
    pub start_addr: usize,
    pub len: usize,
}

impl MemorySpan {
    pub const EMPTY: Self = Self {
        start_addr: 0,
        len: 0,
    };

    pub const fn new(start_addr: usize, len: usize) -> Self {
        Self { start_addr, len }
    }

    pub fn end_addr(self) -> usize {
        self.start_addr.saturating_add(self.len)
    }

    fn is_empty(self) -> bool {
        self.len == 0
    }

    fn merge(self, other: Self) -> Self {
        debug_assert!(
            self.end_addr() >= other.start_addr,
            "merge called on non-overlapping spans"
        );
        
        let start_addr = self.start_addr.min(other.start_addr);
        let end_addr = self.end_addr().max(other.end_addr());

        Self {
            start_addr,
            len: end_addr.saturating_sub(start_addr),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutError {
    BootInfo(BootInfoError),
    MissingMemoryMap,
    NoAvailableMemory,
    TooManyAvailableRegions,
    TooManyReservedRanges,
}

impl From<BootInfoError> for LayoutError {
    fn from(value: BootInfoError) -> Self {
        Self::BootInfo(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryLayout {
    available_regions: [MemorySpan; MAX_AVAILABLE_REGIONS],
    available_count: usize,
    reserved_ranges: [MemorySpan; MAX_RESERVED_RANGES],
    reserved_count: usize,
}

impl MemoryLayout {
    pub fn from_boot_info(
        boot_info_ptr: usize,
        kernel_image: MemorySpan,
    ) -> Result<Self, LayoutError> {
        let info = unsafe { MultibootInfo::parse(boot_info_ptr) }?;
        let (_, memory_map) = info.memory_map().ok_or(LayoutError::MissingMemoryMap)?;

        let mut layout = Self::empty();

        for entry in memory_map {
            if !entry.is_available() {
                continue;
            }

            layout.push_available(MemorySpan::new(
                entry.base_addr as usize,
                entry.length as usize,
            ))?;
        }

        if layout.available_count == 0 {
            return Err(LayoutError::NoAvailableMemory);
        }

        layout.finalize_available_regions();
        layout.push_reserved(MemorySpan::new(0, LOW_MEMORY_RESERVE))?;
        layout.push_reserved(kernel_image)?;
        layout.push_reserved(MemorySpan::new(boot_info_ptr, info.total_size()))?;

        Ok(layout)
    }

    pub fn available_regions(&self) -> &[MemorySpan] {
        &self.available_regions[..self.available_count]
    }

    pub fn reserved_ranges(&self) -> &[MemorySpan] {
        &self.reserved_ranges[..self.reserved_count]
    }

    const fn empty() -> Self {
        Self {
            available_regions: [MemorySpan::EMPTY; MAX_AVAILABLE_REGIONS],
            available_count: 0,
            reserved_ranges: [MemorySpan::EMPTY; MAX_RESERVED_RANGES],
            reserved_count: 0,
        }
    }

    fn push_available(&mut self, span: MemorySpan) -> Result<(), LayoutError> {
        if span.is_empty() {
            return Ok(());
        }

        if self.available_count >= MAX_AVAILABLE_REGIONS {
            return Err(LayoutError::TooManyAvailableRegions);
        }

        self.available_regions[self.available_count] = span;
        self.available_count += 1;
        Ok(())
    }

    fn push_reserved(&mut self, span: MemorySpan) -> Result<(), LayoutError> {
        if span.is_empty() {
            return Ok(());
        }

        if self.reserved_count >= MAX_RESERVED_RANGES {
            return Err(LayoutError::TooManyReservedRanges);
        }

        self.reserved_ranges[self.reserved_count] = span;
        self.reserved_count += 1;
        Ok(())
    }

    fn finalize_available_regions(&mut self) {
        if self.available_count == 0 { return; }
    
        insertion_sort(&mut self.available_regions[..self.available_count]);
    
        let mut write_index = 1;
    
        for read_index in 1..self.available_count {
            let span = self.available_regions[read_index];
            let previous = &mut self.available_regions[write_index - 1];
    
            if previous.end_addr() >= span.start_addr {
                *previous = previous.merge(span);
            } else {
                self.available_regions[write_index] = span;
                write_index += 1;
            }
        }
    
        self.available_count = write_index;
    }
}

fn insertion_sort(spans: &mut [MemorySpan]) {
    for index in 1..spans.len() {
        let current = spans[index];
        let mut slot = index;

        while slot > 0 && spans[slot - 1].start_addr > current.start_addr {
            spans[slot] = spans[slot - 1];
            slot -= 1;
        }

        spans[slot] = current;
    }
}
