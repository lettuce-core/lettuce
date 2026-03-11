use core::fmt::Write;

pub struct FixedBuf<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl Write for FixedBuf<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            if self.pos >= self.buf.len() {
                return Err(core::fmt::Error);
            }
            
            self.buf[self.pos] = b;
            self.pos += 1;
        }
        
        Ok(())
    }
}

impl<'a> FixedBuf<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    
    pub fn as_str(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(&self.buf[..self.pos])
        }
    }
    
    pub fn into_str(self) -> &'a str {
        unsafe {
            core::str::from_utf8_unchecked(&self.buf[..self.pos])
        }
    }
}
