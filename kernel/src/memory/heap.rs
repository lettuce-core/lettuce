use core::{
    alloc::Layout,
    cell::UnsafeCell,
    ptr::{self, NonNull},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub const EARLY_HEAP_CAPACITY: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeapStats {
    pub capacity_bytes: usize,
    pub used_bytes: usize,
    pub free_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeapError {
    NotInitialized,
    AlreadyInitialized,
    OutOfMemory,
    InvalidLayout,
}

#[repr(align(16))]
struct HeapArena([u8; EARLY_HEAP_CAPACITY]);

struct HeapArenaCell(UnsafeCell<HeapArena>);

unsafe impl Sync for HeapArenaCell {}

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static NEXT_OFFSET: AtomicUsize = AtomicUsize::new(0);
static HEAP_ARENA: HeapArenaCell = HeapArenaCell(UnsafeCell::new(HeapArena([0; EARLY_HEAP_CAPACITY])));

pub fn init() -> Result<(), HeapError> {
    if INITIALIZED.swap(true, Ordering::AcqRel) {
        return Err(HeapError::AlreadyInitialized);
    }

    NEXT_OFFSET.store(0, Ordering::Release);
    Ok(())
}

pub fn alloc(layout: Layout) -> Result<NonNull<u8>, HeapError> {
    ensure_initialized()?;

    if layout.align() == 0 || !layout.align().is_power_of_two() {
        return Err(HeapError::InvalidLayout);
    }

    let size = layout.size();

    loop {
        let current = NEXT_OFFSET.load(Ordering::Acquire);
        let start = align_up(current, layout.align()).ok_or(HeapError::InvalidLayout)?;
        let end = start.checked_add(size).ok_or(HeapError::OutOfMemory)?;

        if end > EARLY_HEAP_CAPACITY {
            return Err(HeapError::OutOfMemory);
        }

        if NEXT_OFFSET
            .compare_exchange(current, end, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let ptr = unsafe { heap_base().add(start) };
            return Ok(unsafe { NonNull::new_unchecked(ptr) });
        }
    }
}

pub fn alloc_zeroed(layout: Layout) -> Result<NonNull<u8>, HeapError> {
    let ptr = alloc(layout)?;

    if layout.size() != 0 {
        unsafe {
            ptr::write_bytes(ptr.as_ptr(), 0, layout.size());
        }
    }

    Ok(ptr)
}

pub fn stats() -> Result<HeapStats, HeapError> {
    ensure_initialized()?;

    let used_bytes = NEXT_OFFSET.load(Ordering::Acquire).min(EARLY_HEAP_CAPACITY);

    Ok(HeapStats {
        capacity_bytes: EARLY_HEAP_CAPACITY,
        used_bytes,
        free_bytes: EARLY_HEAP_CAPACITY.saturating_sub(used_bytes),
    })
}

fn ensure_initialized() -> Result<(), HeapError> {
    if !INITIALIZED.load(Ordering::Acquire) {
        return Err(HeapError::NotInitialized);
    }

    Ok(())
}

fn align_up(value: usize, align: usize) -> Option<usize> {
    value.checked_add(align - 1).map(|v| v & !(align - 1))
}

unsafe fn heap_base() -> *mut u8 {
    let arena = &mut *HEAP_ARENA.0.get();
    arena.0.as_mut_ptr()
}
