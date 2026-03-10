#![allow(dead_code)]

use crate::memory::pmm::{PhysFrame, PAGE_SIZE};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageCount(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapFlags(u64);

impl MapFlags {
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER: Self = Self(1 << 2);
    pub const NO_EXEC: Self = Self(1 << 3);

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VmmError {
    NotInitialized,
    InvalidAddress,
    InvalidLength,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmmInitReport {
    pub page_size: usize,
}

impl VmmInitReport {
    pub fn label(self) -> &'static str {
        "memory: vmm initialized (skeleton)"
    }
}

static mut INITIALIZED: bool = false;

pub fn init() -> VmmInitReport {
    unsafe {
        INITIALIZED = true;
    }

    VmmInitReport {
        page_size: PAGE_SIZE,
    }
}

pub fn map_pages(
    virt_start: VirtAddr,
    phys_start: PhysFrame,
    pages: PageCount,
    flags: MapFlags,
) -> Result<(), VmmError> {
    ensure_initialized()?;
    validate_range(virt_start, pages)?;

    arch_map_pages(virt_start, phys_start, pages, flags)
}

pub fn unmap_pages(virt_start: VirtAddr, pages: PageCount) -> Result<(), VmmError> {
    ensure_initialized()?;
    validate_range(virt_start, pages)?;

    arch_unmap_pages(virt_start, pages)
}

fn ensure_initialized() -> Result<(), VmmError> {
    let initialized = unsafe { INITIALIZED };
    
    if !initialized {
        return Err(VmmError::NotInitialized);
    }
    
    Ok(())
}

fn validate_range(virt_start: VirtAddr, pages: PageCount) -> Result<(), VmmError> {
    if pages.0 == 0 {
        return Err(VmmError::InvalidLength);
    }
    
    if virt_start.0 % PAGE_SIZE != 0 {
        return Err(VmmError::InvalidAddress);
    }
    
    Ok(())
}

fn arch_map_pages(
    _virt_start: VirtAddr,
    _phys_start: PhysFrame,
    _pages: PageCount,
    _flags: MapFlags,
) -> Result<(), VmmError> {
    Err(VmmError::Unsupported)
}

fn arch_unmap_pages(_virt_start: VirtAddr, _pages: PageCount) -> Result<(), VmmError> {
    Err(VmmError::Unsupported)
}
