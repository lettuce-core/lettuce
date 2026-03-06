pub const MULTIBOOT2_MAGIC: u32 = 0x36d7_6289;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootSource {
    GrubMultiboot2,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootReport {
    pub source: BootSource,
    pub magic: u32,
}

impl BootReport {
    pub fn detect(boot_magic: u32) -> Self {
        let source = if boot_magic == MULTIBOOT2_MAGIC {
            BootSource::GrubMultiboot2
        } else {
            BootSource::Unknown
        };

        Self {
            source,
            magic: boot_magic,
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self.source {
            BootSource::GrubMultiboot2 => "boot source: grub multiboot2",
            BootSource::Unknown => "boot source: unknown",
        }
    }

    pub fn validation_label(&self) -> &'static str {
        match self.source {
            BootSource::GrubMultiboot2 => "boot validation: magic matched",
            BootSource::Unknown => "boot validation: magic mismatch",
        }
    }
}
