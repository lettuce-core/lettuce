use core::sync::atomic::{AtomicBool, Ordering};

const PAGE_SIZE: usize = 4096;
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
pub struct PhysFrame {
    index: usize,
}

impl PhysFrame {
    pub fn _index(self) -> usize {
        self.index
    }

    pub fn _addr(self) -> usize {
        self.index * PAGE_SIZE
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

        for word in &mut FRAME_BITMAP {
            *word = 0;
        }

        for frame_idx in 0..config.reserved_frames {
            bitmap_set(frame_idx);
        }
    }

    INITIALIZED.store(true, Ordering::Release);
}

pub fn _alloc_frame() -> Result<PhysFrame, PmmError> {
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

pub fn _free_frame(frame: PhysFrame) -> Result<(), PmmError> {
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
        let words = bitmap_words_for_total(TOTAL_FRAMES);
        let mut used = 0usize;

        for i in 0..words {
            used += FRAME_BITMAP[i].count_ones() as usize;
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
    (total_frames + 63) / 64
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
