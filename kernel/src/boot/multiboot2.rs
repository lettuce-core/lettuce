use crate::utils::align_up;
use core::slice;

const HEADER_SIZE: usize = 8;
const ALIGN: usize = 8;
const MAX_BOOTINFO_SIZE: usize = 64 * 1024;

const TAG_END: u32 = 0;
const TAG_MMAP: u32 = 6;
const TAG_FRAMEBUFFER: u32 = 8;
const MMAP_TYPE_AVAILABLE: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootInfoError {
    NullPointer,
    MisalignedPointer,
    InvalidSize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultibootInfo<'a> {
    bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tag<'a> {
    pub typ: u32,
    pub payload: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootInfoSummary {
    pub has_mmap: bool,
    pub has_framebuffer: bool,
    pub tag_count: usize,
}

impl BootInfoSummary {
    pub fn label(&self) -> &'static str {
        match (self.has_mmap, self.has_framebuffer) {
            (true, true) => "boot info: mmap + framebuffer tags found",
            (true, false) => "boot info: mmap tag found",
            (false, true) => "boot info: framebuffer tag found",
            (false, false) => "boot info: parsed, no mmap/framebuffer tags",
        }
    }
}

pub struct TagIter<'a> {
    rem: &'a [u8],
    done: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryMapHeader {
    pub entry_size: u32,
    pub entry_version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryMapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub typ: u32,
}

impl MemoryMapEntry {
    pub fn is_available(self) -> bool {
        self.typ == MMAP_TYPE_AVAILABLE
    }
}

pub struct MemoryMapIter<'a> {
    rem: &'a [u8],
    entry_size: usize,
}

impl<'a> MultibootInfo<'a> {
    pub unsafe fn parse(ptr: usize) -> Result<Self, BootInfoError> {
        if ptr == 0 {
            return Err(BootInfoError::NullPointer);
        }
        
        if ptr % ALIGN != 0 {
            return Err(BootInfoError::MisalignedPointer);
        }

        let total_size = read_u32(ptr as *const u8) as usize;
        if !(HEADER_SIZE..=MAX_BOOTINFO_SIZE).contains(&total_size) {
            return Err(BootInfoError::InvalidSize);
        }

        let bytes = unsafe { slice::from_raw_parts(ptr as *const u8, total_size) };
        Ok(Self { bytes })
    }

    pub fn tags(&self) -> TagIter<'a> {
        TagIter {
            rem: &self.bytes[HEADER_SIZE..],
            done: false,
        }
    }

    pub fn summary(&self) -> BootInfoSummary {
        let mut has_mmap = false;
        let mut has_framebuffer = false;
        let mut tag_count = 0usize;

        for tag in self.tags() {
            tag_count += 1;
            
            match tag.typ {
                TAG_MMAP => has_mmap = true,
                TAG_FRAMEBUFFER => has_framebuffer = true,
                _ => {}
            }
        }

        BootInfoSummary {
            has_mmap,
            has_framebuffer,
            tag_count,
        }
    }

    pub fn total_size(&self) -> usize {
        self.bytes.len()
    }

    pub fn memory_map(&self) -> Option<(MemoryMapHeader, MemoryMapIter<'a>)> {
        for tag in self.tags() {
            if tag.typ != TAG_MMAP {
                continue;
            }

            if tag.payload.len() < 8 {
                return None;
            }
            
            let entry_size = read_u32(&tag.payload[0]);
            let entry_version = read_u32(&tag.payload[4]);
            let entries = &tag.payload[8..];

            if entry_size < 24 {
                return None;
            }

            return Some((
                MemoryMapHeader {
                    entry_size,
                    entry_version,
                },
                
                MemoryMapIter {
                    rem: entries,
                    entry_size: entry_size as usize,
                },
            ));
        }

        None
    }
}

impl<'a> Iterator for TagIter<'a> {
    type Item = Tag<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.rem.len() < HEADER_SIZE {
            return None;
        }

        let typ = read_u32(self.rem.as_ptr());
        let size = read_u32(unsafe { self.rem.as_ptr().add(4) }) as usize;

        if size < HEADER_SIZE || size > self.rem.len() {
            self.done = true;
            return None;
        }

        if typ == TAG_END {
            self.done = true;
            return None;
        }

        let payload = &self.rem[HEADER_SIZE..size];
        let Some(step) = align_up(size, ALIGN) else {
            self.done = true;
            return None;
        };

        self.rem = if step <= self.rem.len() {
            &self.rem[step..]
        } else {
            &[]
        };

        Some(Tag { typ, payload })
    }
}

impl<'a> Iterator for MemoryMapIter<'a> {
    type Item = MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entry_size == 0 || self.rem.len() < self.entry_size {
            return None;
        }

        let raw = &self.rem[..self.entry_size];
        self.rem = &self.rem[self.entry_size..];

        if raw.len() < 24 {
            return None;
        }

        Some(MemoryMapEntry {
            base_addr: read_u64(raw.as_ptr()),
            length: read_u64(unsafe { raw.as_ptr().add(8) }),
            typ: read_u32(unsafe { raw.as_ptr().add(16) }),
        })
    }
}

fn read_u32(ptr: *const u8) -> u32 {
    u32::from_le(unsafe { (ptr as *const u32).read_unaligned() })
}

fn read_u64(ptr: *const u8) -> u64 {
    u64::from_le(unsafe { (ptr as *const u64).read_unaligned() })
}
