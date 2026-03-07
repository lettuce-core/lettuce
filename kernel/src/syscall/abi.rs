#![allow(dead_code)]

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallNumber {
    MemAllocFrame = 0x1000,
    MemMapPages = 0x1001,
    MemUnmapPages = 0x1002,
}

impl TryFrom<u64> for SyscallNumber {
    type Error = SyscallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            x if x == SyscallNumber::MemAllocFrame as u64 => Ok(SyscallNumber::MemAllocFrame),
            x if x == SyscallNumber::MemMapPages as u64 => Ok(SyscallNumber::MemMapPages),
            x if x == SyscallNumber::MemUnmapPages as u64 => Ok(SyscallNumber::MemUnmapPages),
            _ => Err(SyscallError::InvalidSyscall),
        }
    }
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallError {
    InvalidSyscall = 1,
    InvalidArg = 2,
    OutOfMemory = 3,
    Unsupported = 4,
    NotReady = 5,
    Internal = 255,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallResult {
    pub value: u64,
    pub error: u64,
}

impl SyscallResult {
    pub const fn ok(value: u64) -> Self {
        Self { value, error: 0 }
    }

    pub const fn err(error: SyscallError) -> Self {
        Self {
            value: 0,
            error: error as u64,
        }
    }
}
