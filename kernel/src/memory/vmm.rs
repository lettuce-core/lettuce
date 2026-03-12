#![allow(dead_code)]

use crate::memory::pmm::{PhysFrame, PAGE_SIZE};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// :: todo: derive this from the real boot page tables
const BOOT_IDENTITY_MAP_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    pub const fn is_page_aligned(self) -> bool {
        self.0 % PAGE_SIZE == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    pub const fn is_page_aligned(self) -> bool {
        self.0 % PAGE_SIZE == 0
    }

    pub fn checked_add_pages(self, pages: PageCount) -> Option<Self> {
        let bytes = pages.0.checked_mul(PAGE_SIZE)?;
        self.0.checked_add(bytes).map(Self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageCount(pub usize);

impl PageCount {
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn from_addr(addr: VirtAddr) -> Result<Self, VmmError> {
        if !addr.is_page_aligned() {
            return Err(VmmError::InvalidAddress);
        }

        Ok(Self {
            number: addr.0 / PAGE_SIZE,
        })
    }

    pub fn start_addr(self) -> VirtAddr {
        VirtAddr(self.number * PAGE_SIZE)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageRange {
    start: Page,
    count: PageCount,
}

impl PageRange {
    pub fn new(virt_start: VirtAddr, pages: PageCount) -> Result<Self, VmmError> {
        if pages.is_zero() {
            return Err(VmmError::InvalidLength);
        }

        let start = Page::from_addr(virt_start)?;
        virt_start
            .checked_add_pages(pages)
            .ok_or(VmmError::InvalidLength)?;

        Ok(Self { start, count: pages })
    }

    pub fn start_addr(self) -> VirtAddr {
        self.start.start_addr()
    }

    pub const fn page_count(self) -> PageCount {
        self.count
    }
}

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
pub struct MappingRequest {
    pub virt_start: VirtAddr,
    pub phys_start: PhysFrame,
    pub pages: PageCount,
    pub flags: MapFlags,
}

impl MappingRequest {
    pub fn new(
        virt_start: VirtAddr,
        phys_start: PhysFrame,
        pages: PageCount,
        flags: MapFlags,
    ) -> Result<Self, VmmError> {
        let _ = PageRange::new(virt_start, pages)?;

        let phys_end = pages.0
            .checked_mul(PAGE_SIZE)
            .and_then(|bytes| phys_start.addr().checked_add(bytes))
            .ok_or(VmmError::InvalidLength)?;
        
        let _ = phys_end;

        Ok(Self {
            virt_start,
            phys_start,
            pages,
            flags,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressSpace {
    root_table: PhysAddr,
    identity_map_bytes: usize,
}

impl AddressSpace {
    pub const fn root_table(self) -> PhysAddr {
        self.root_table
    }

    pub const fn identity_map_bytes(self) -> usize {
        self.identity_map_bytes
    }

    pub fn is_identity_mapped(self, addr: VirtAddr) -> bool {
        addr.0 < self.identity_map_bytes
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
    pub kernel_root_table: PhysAddr,
    pub identity_map_bytes: usize,
}

impl VmmInitReport {
    pub fn label(self) -> &'static str {
        "memory: vmm initialized from boot paging state"
    }
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);
// identity_map_bytes is a constant, so only root_table needs to be stored
static KERNEL_ROOT_TABLE: AtomicUsize = AtomicUsize::new(0);

pub fn init() -> VmmInitReport {
    let root = read_cr3_arch();
    KERNEL_ROOT_TABLE.store(root, Ordering::Release);
    INITIALIZED.store(true, Ordering::Release);

    VmmInitReport {
        page_size: PAGE_SIZE,
        kernel_root_table: PhysAddr(root),
        identity_map_bytes: BOOT_IDENTITY_MAP_BYTES,
    }
}

pub fn kernel_address_space() -> Result<AddressSpace, VmmError> {
    ensure_initialized()?;

    Ok(AddressSpace {
        root_table: PhysAddr(KERNEL_ROOT_TABLE.load(Ordering::Acquire)),
        identity_map_bytes: BOOT_IDENTITY_MAP_BYTES,
    })
}

pub fn map_pages(
    virt_start: VirtAddr,
    phys_start: PhysFrame,
    pages: PageCount,
    flags: MapFlags,
) -> Result<(), VmmError> {
    let s = kernel_address_space()?;
    let req = MappingRequest::new(virt_start, phys_start, pages, flags)?;
    arch_map_pages(s, req)
}

pub fn unmap_pages(virt_start: VirtAddr, pages: PageCount) -> Result<(), VmmError> {
    let s = kernel_address_space()?;
    let r = PageRange::new(virt_start, pages)?;
    arch_unmap_pages(s, r)
}

pub fn identity_map_addr(phys_addr: PhysAddr) -> Result<VirtAddr, VmmError> {
    let s = kernel_address_space()?;
    let va = VirtAddr(phys_addr.0);

    if !s.is_identity_mapped(va) {
        return Err(VmmError::Unsupported);
    }

    Ok(va)
}

fn ensure_initialized() -> Result<(), VmmError> {
    if !INITIALIZED.load(Ordering::Acquire) {
        return Err(VmmError::NotInitialized);
    }

    Ok(())
}

fn arch_map_pages(
    _address_space: AddressSpace,
    _request: MappingRequest,
) -> Result<(), VmmError> {
    Err(VmmError::Unsupported)
}

fn arch_unmap_pages(_address_space: AddressSpace, _range: PageRange) -> Result<(), VmmError> {
    Err(VmmError::Unsupported)
}

#[cfg(target_arch = "x86_64")]
fn read_cr3_arch() -> usize {
    let v: usize;

    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) v, options(nomem, nostack, preserves_flags));
    }

    v
}

#[cfg(not(target_arch = "x86_64"))]
fn read_cr3_arch() -> usize {
    0
}
