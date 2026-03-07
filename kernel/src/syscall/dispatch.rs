#![allow(dead_code)]

use crate::memory::{
    pmm,
    vmm::{self, MapFlags, PageCount, VmmError, VirtAddr},
};

use super::abi::{SyscallError, SyscallNumber, SyscallResult};

pub const REGISTERED_SYSCALLS: usize = 3;

pub fn dispatch(number: u64, args: [u64; 6]) -> SyscallResult {
    let syscall = match SyscallNumber::try_from(number) {
        Ok(s) => s,
        Err(e) => return SyscallResult::err(e),
    };

    match syscall {
        SyscallNumber::MemAllocFrame => sys_mem_alloc_frame(),
        SyscallNumber::MemMapPages => sys_mem_map_pages(args),
        SyscallNumber::MemUnmapPages => sys_mem_unmap_pages(args),
    }
}

fn sys_mem_alloc_frame() -> SyscallResult {
    match pmm::alloc_frame() {
        Ok(frame) => SyscallResult::ok(frame.addr() as u64),
        Err(pmm::PmmError::OutOfMemory) => SyscallResult::err(SyscallError::OutOfMemory),
        Err(pmm::PmmError::NotInitialized) => SyscallResult::err(SyscallError::NotReady),
        Err(_) => SyscallResult::err(SyscallError::Internal),
    }
}

fn sys_mem_map_pages(args: [u64; 6]) -> SyscallResult {
    let virt_start = args[0] as usize;
    let phys_start = args[1] as usize;
    let pages = args[2] as usize;
    let flags = args[3];

    let Some(frame) = pmm::PhysFrame::from_addr(phys_start) else {
        return SyscallResult::err(SyscallError::InvalidArg);
    };

    match vmm::map_pages(
        VirtAddr(virt_start),
        frame,
        PageCount(pages),
        MapFlags::from_bits(flags),
    ) {
        Ok(()) => SyscallResult::ok(0),
        Err(e) => SyscallResult::err(map_vmm_error(e)),
    }
}

fn sys_mem_unmap_pages(args: [u64; 6]) -> SyscallResult {
    let virt_start = args[0] as usize;
    let pages = args[1] as usize;

    match vmm::unmap_pages(VirtAddr(virt_start), PageCount(pages)) {
        Ok(()) => SyscallResult::ok(0),
        Err(e) => SyscallResult::err(map_vmm_error(e)),
    }
}

fn map_vmm_error(err: VmmError) -> SyscallError {
    match err {
        VmmError::NotInitialized => SyscallError::NotReady,
        VmmError::InvalidAddress | VmmError::InvalidLength => SyscallError::InvalidArg,
        VmmError::Unsupported => SyscallError::Unsupported,
    }
}
