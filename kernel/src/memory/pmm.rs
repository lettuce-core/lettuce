#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, Ordering};

pub const PAGE_SIZE: usize = 4096;
pub const MAX_FRAMES: usize = 32 * 1024;
const BITMAP_WORDS: usize = MAX_FRAMES / 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EarlyPmmConfig {
    pub total_frames: usize,
    pub reserved_frames: usize,
}

impl Default for EarlyPmmConfig {
    fn default() -> Self {
        Self {
            total_frames: 16 * 1024,
            reserved_frames: 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRegion {
    pub start_addr: usize,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReservedRange {
    pub start_addr: usize,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PmmInitInput<'a> {
    pub available_regions: &'a [MemoryRegion],
    pub reserved_ranges: &'a [ReservedRange],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysFrame {
    index: usize,
}

impl PhysFrame {
    pub fn index(self) -> usize {
        self.index
    }

    pub fn addr(self) -> usize {
        self.index * PAGE_SIZE
    }

    pub fn from_addr(addr: usize) -> Option<Self> {
        if addr % PAGE_SIZE != 0 {
            return None;
        }

        let index = addr / PAGE_SIZE;
        if index >= MAX_FRAMES {
            return None;
        }

        Some(Self { index })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PmmStats {
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PmmError {
    NotInitialized,
    AlreadyInitialized,
    InvalidConfig,
    OutOfMemory,
    DoubleFree,
    OutOfRange,
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

static mut TOTAL_FRAMES: usize = 0;
static mut FRAME_BITMAP: [u64; BITMAP_WORDS] = [0; BITMAP_WORDS];

pub fn init(config: EarlyPmmConfig) {
    if INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    assert!(config.total_frames > 0);
    assert!(config.total_frames <= MAX_FRAMES);
    assert!(config.reserved_frames <= config.total_frames);

    unsafe {
        TOTAL_FRAMES = config.total_frames;
        set_all_used(TOTAL_FRAMES);
        clear_frames(0, config.total_frames);
        set_frames(0, config.reserved_frames);
    }

    INITIALIZED.store(true, Ordering::Release);
}

pub fn init_from_memory_map(input: PmmInitInput<'_>) -> Result<(), PmmError> {
    if INITIALIZED.load(Ordering::Acquire) {
        return Err(PmmError::AlreadyInitialized);
    }

    let mut frame_limit = 0usize;

    for region in input.available_regions {
        let end = region
            .start_addr
            .saturating_add(region.len)
            .min(MAX_FRAMES * PAGE_SIZE);
        frame_limit = frame_limit.max(end / PAGE_SIZE);
    }

    if frame_limit == 0 {
        return Err(PmmError::InvalidConfig);
    }

    unsafe {
        TOTAL_FRAMES = frame_limit;
        set_all_used(TOTAL_FRAMES);

        for region in input.available_regions {
            if let Some((start_frame, end_frame)) =
                addr_range_to_frames(region.start_addr, region.len)
            {
                clear_frames(start_frame, end_frame);
            }
        }

        for reserved in input.reserved_ranges {
            if let Some((start_frame, end_frame)) =
                addr_range_to_frames(reserved.start_addr, reserved.len)
            {
                set_frames(start_frame, end_frame);
            }
        }
    }

    INITIALIZED.store(true, Ordering::Release);
    Ok(())
}

pub fn alloc_frame() -> Result<PhysFrame, PmmError> {
    ensure_initialized()?;

    unsafe {
        for word_idx in 0..bitmap_words_for_total(TOTAL_FRAMES) {
            let word = FRAME_BITMAP[word_idx];
            if word == u64::MAX {
                continue;
            }

            let bit_idx = (!word).trailing_zeros() as usize;
            let frame_idx = word_idx * 64 + bit_idx;

            if frame_idx >= TOTAL_FRAMES {
                return Err(PmmError::OutOfMemory);
            }

            bitmap_set(frame_idx);
            return Ok(PhysFrame { index: frame_idx });
        }
    }

    Err(PmmError::OutOfMemory)
}

pub fn free_frame(frame: PhysFrame) -> Result<(), PmmError> {
    ensure_initialized()?;

    unsafe {
        if frame.index >= TOTAL_FRAMES {
            return Err(PmmError::OutOfRange);
        }
        if !bitmap_test(frame.index) {
            return Err(PmmError::DoubleFree);
        }

        bitmap_clear(frame.index);
    }

    Ok(())
}

pub fn stats() -> Result<PmmStats, PmmError> {
    ensure_initialized()?;

    unsafe {
        let mut used = 0usize;

        for frame in 0..TOTAL_FRAMES {
            if bitmap_test(frame) {
                used += 1;
            }
        }

        Ok(PmmStats {
            total_frames: TOTAL_FRAMES,
            used_frames: used,
            free_frames: TOTAL_FRAMES - used,
        })
    }
}

fn ensure_initialized() -> Result<(), PmmError> {
    if !INITIALIZED.load(Ordering::Acquire) {
        return Err(PmmError::NotInitialized);
    }
    Ok(())
}

fn bitmap_words_for_total(total_frames: usize) -> usize {
    total_frames.div_ceil(64)
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + (align - 1)) & !(align - 1)
}

fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

fn addr_range_to_frames(start_addr: usize, len: usize) -> Option<(usize, usize)> {
    if len == 0 {
        return None;
    }

    let limit_addr = MAX_FRAMES * PAGE_SIZE;
    let start = align_up(start_addr, PAGE_SIZE).min(limit_addr);
    let end = align_down(start_addr.saturating_add(len), PAGE_SIZE).min(limit_addr);

    if start >= end {
        return None;
    }

    Some((start / PAGE_SIZE, end / PAGE_SIZE))
}

unsafe fn set_all_used(total_frames: usize) {
    let words = bitmap_words_for_total(total_frames);
    for i in 0..words {
        FRAME_BITMAP[i] = u64::MAX;
    }
    for i in words..BITMAP_WORDS {
        FRAME_BITMAP[i] = 0;
    }
}

unsafe fn clear_frames(start_frame: usize, end_frame: usize) {
    let capped_end = end_frame.min(TOTAL_FRAMES);
    for frame in start_frame.min(capped_end)..capped_end {
        bitmap_clear(frame);
    }
}

unsafe fn set_frames(start_frame: usize, end_frame: usize) {
    let capped_end = end_frame.min(TOTAL_FRAMES);
    for frame in start_frame.min(capped_end)..capped_end {
        bitmap_set(frame);
    }
}

unsafe fn bitmap_set(frame_idx: usize) {
    let (word_idx, bit_idx) = split_index(frame_idx);
    FRAME_BITMAP[word_idx] |= 1u64 << bit_idx;
}

unsafe fn bitmap_clear(frame_idx: usize) {
    let (word_idx, bit_idx) = split_index(frame_idx);
    FRAME_BITMAP[word_idx] &= !(1u64 << bit_idx);
}

unsafe fn bitmap_test(frame_idx: usize) -> bool {
    let (word_idx, bit_idx) = split_index(frame_idx);
    (FRAME_BITMAP[word_idx] & (1u64 << bit_idx)) != 0
}

fn split_index(frame_idx: usize) -> (usize, usize) {
    (frame_idx / 64, frame_idx % 64)
}
