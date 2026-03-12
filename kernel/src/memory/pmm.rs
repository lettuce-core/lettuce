use super::layout::{MemoryLayout, MemorySpan};
use crate::utils::{align_down, align_up};
use core::sync::atomic::{AtomicBool, Ordering};

pub const PAGE_SIZE: usize = 4096;
pub const MAX_FRAMES: usize = 32 * 1024;
const BITMAP_WORDS: usize = MAX_FRAMES / 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AvailableFrameSummary {
    frame_limit: usize,
    usable_frames: usize,
}

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
pub struct PhysFrame {
    index: usize,
}

impl PhysFrame {
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
    pub tracked_frames: usize,
    pub usable_frames: usize,
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
static mut USABLE_FRAMES: usize = 0;
static mut FRAME_BITMAP: [u64; BITMAP_WORDS] = [0; BITMAP_WORDS];

pub fn init(config: EarlyPmmConfig) -> Result<(), PmmError> {
    if INITIALIZED.load(Ordering::Acquire) {
        return Err(PmmError::AlreadyInitialized);
    }

    if config.total_frames == 0
        || config.total_frames > MAX_FRAMES
        || config.reserved_frames > config.total_frames
    {
        return Err(PmmError::InvalidConfig);
    }

    unsafe {
        TOTAL_FRAMES = config.total_frames;
        USABLE_FRAMES = config.total_frames;
        
        set_all_used(TOTAL_FRAMES);
        clear_frames(0, config.total_frames);
        set_frames(0, config.reserved_frames);
    }

    INITIALIZED.store(true, Ordering::Release);
    Ok(())
}

pub fn init_from_layout(layout: &MemoryLayout) -> Result<(), PmmError> {
    if INITIALIZED.load(Ordering::Acquire) {
        return Err(PmmError::AlreadyInitialized);
    }

    let available = validate_available_ranges(layout.available_regions())?;

    unsafe {
        TOTAL_FRAMES = available.frame_limit;
        USABLE_FRAMES = available.usable_frames;
        set_all_used(TOTAL_FRAMES);

        for span in layout.available_regions() {
            if let Some((start_frame, end_frame)) = span_to_frame_range(*span) {
                clear_frames(start_frame, end_frame);
            }
        }

        for span in layout.reserved_ranges() {
            if let Some((start_frame, end_frame)) = span_to_frame_range(*span) {
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
        let free_frames = count_free_frames(TOTAL_FRAMES);
        let used_frames = USABLE_FRAMES.saturating_sub(free_frames);

        Ok(PmmStats {
            tracked_frames: TOTAL_FRAMES,
            usable_frames: USABLE_FRAMES,
            used_frames,
            free_frames,
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

fn span_to_frame_range(span: MemorySpan) -> Option<(usize, usize)> {
    if span.len == 0 {
        return None;
    }

    let limit_addr = MAX_FRAMES * PAGE_SIZE;
    let start = align_up(span.start_addr, PAGE_SIZE)?.min(limit_addr);
    let end = align_down(span.end_addr(), PAGE_SIZE).min(limit_addr);

    if start >= end {
        return None;
    }

    Some((start / PAGE_SIZE, end / PAGE_SIZE))
}

fn validate_available_ranges(spans: &[MemorySpan]) -> Result<AvailableFrameSummary, PmmError> {
    let mut previous_end = 0usize;
    let mut frame_limit = 0usize;
    let mut usable_frames = 0usize;
    let mut saw_range = false;

    for span in spans {
        let (start_frame, end_frame) = span_to_frame_range(*span).ok_or(PmmError::InvalidConfig)?;

        if saw_range && start_frame < previous_end {
            return Err(PmmError::InvalidConfig);
        }

        usable_frames = usable_frames.saturating_add(end_frame.saturating_sub(start_frame));
        frame_limit = frame_limit.max(end_frame);
        previous_end = end_frame;
        saw_range = true;
    }

    if !saw_range || usable_frames == 0 || frame_limit == 0 {
        return Err(PmmError::InvalidConfig);
    }

    Ok(AvailableFrameSummary {
        frame_limit,
        usable_frames,
    })
}

unsafe fn count_free_frames(total_frames: usize) -> usize {
    FRAME_BITMAP[..bitmap_words_for_total(total_frames)]
        .iter()
        .map(|w| w.count_zeros() as usize)
        .sum()
}

unsafe fn clear_frames(start_frame: usize, end_frame: usize) {
    apply_frames(
        start_frame, 
        end_frame, 
        |word, mask| 
        word & !mask
    )
}

unsafe fn set_frames(start_frame: usize, end_frame: usize) {
    apply_frames(
        start_frame, 
        end_frame, 
        |word, mask| 
        word | mask
    )
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

unsafe fn apply_frames(
    start_frame: usize,
    end_frame: usize,
    op: impl Fn(u64, u64) -> u64,
) {
    let start = start_frame.min(TOTAL_FRAMES);
    let end = end_frame.min(TOTAL_FRAMES);

    if start >= end {
        return;
    }

    let start_word = start / 64;
    let end_word = (end - 1) / 64;

    if start_word == end_word {
        let mask = word_mask(start % 64, end % 64);
        FRAME_BITMAP[start_word] = op(FRAME_BITMAP[start_word], mask);
        
        return;
    }

    let head_mask = word_mask(start % 64, 64);
    FRAME_BITMAP[start_word] = op(FRAME_BITMAP[start_word], head_mask);
    
    for word in &mut FRAME_BITMAP[start_word + 1..end_word] {
        *word = op(*word, u64::MAX);
    }

    let tail_mask = word_mask(0, end % 64);
    if tail_mask != 0 {
        FRAME_BITMAP[end_word] = op(FRAME_BITMAP[end_word], tail_mask);
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

// produces a bitmask with 1s in bit positions [start, end)
// (both values must be in 0..=64)
// 
fn word_mask(start: usize, end: usize) -> u64 {
    if end == 64 {
        u64::MAX << start
    } else {
        ((1u64 << end) - 1) & !((1u64 << start) - 1)
    }
}
