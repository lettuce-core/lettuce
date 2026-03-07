use core::{mem, slice};

const HEADER_SIZE: usize = 8;
const ALIGN: usize = 8;
const MAX_BOOTINFO_SIZE: usize = 64 * 1024;

const TAG_END: u32 = 0;
const TAG_MMAP: u32 = 6;
const TAG_FRAMEBUFFER: u32 = 8;

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

            if tag.typ == TAG_MMAP {
                has_mmap = true;
            } else if tag.typ == TAG_FRAMEBUFFER {
                has_framebuffer = true;
            }
        }

        BootInfoSummary {
            has_mmap,
            has_framebuffer,
            tag_count,
        }
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
        let step = align_up(size, ALIGN);

        self.rem = if step <= self.rem.len() {
            &self.rem[step..]
        } else {
            &[]
        };

        Some(Tag { typ, payload })
    }
}

fn align_up(value: usize, align: usize) -> usize {
    (value + (align - 1)) & !(align - 1)
}

fn read_u32(ptr: *const u8) -> u32 {
    let mut bytes = [0u8; mem::size_of::<u32>()];

    unsafe {
        bytes.copy_from_slice(slice::from_raw_parts(ptr, mem::size_of::<u32>()));
    }

    u32::from_le_bytes(bytes)
}
