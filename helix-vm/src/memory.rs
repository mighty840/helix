use thiserror::Error;

/// 16-bit word-addressed memory, 1024 words (2 KB).
pub const MEM_SIZE: usize = 1024;
pub const STACK_INIT: u16 = MEM_SIZE as u16; // SP starts at top (empty stack)

#[derive(Debug, Error)]
pub enum MemError {
    #[error("memory address {0:#06x} out of bounds")]
    OutOfBounds(u16),
    #[error("stack overflow")]
    StackOverflow,
    #[error("stack underflow")]
    StackUnderflow,
}

pub struct Memory {
    data: [u16; MEM_SIZE],
    pub sp: u16,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            data: [0u16; MEM_SIZE],
            sp: STACK_INIT,
        }
    }

    pub fn load(&self, addr: u16) -> Result<u16, MemError> {
        let idx = addr as usize;
        if idx >= MEM_SIZE {
            return Err(MemError::OutOfBounds(addr));
        }
        Ok(self.data[idx])
    }

    pub fn store(&mut self, addr: u16, val: u16) -> Result<(), MemError> {
        let idx = addr as usize;
        if idx >= MEM_SIZE {
            return Err(MemError::OutOfBounds(addr));
        }
        self.data[idx] = val;
        Ok(())
    }

    pub fn push(&mut self, val: u16) -> Result<(), MemError> {
        if self.sp == 0 {
            return Err(MemError::StackOverflow);
        }
        self.sp -= 1;
        self.data[self.sp as usize] = val;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<u16, MemError> {
        if self.sp >= STACK_INIT {
            return Err(MemError::StackUnderflow);
        }
        let val = self.data[self.sp as usize];
        self.sp += 1;
        Ok(val)
    }

    /// Load a slice of words into memory starting at address 0.
    pub fn load_data(&mut self, data: &[u16]) {
        for (i, &v) in data.iter().enumerate().take(MEM_SIZE) {
            self.data[i] = v;
        }
    }

    /// Return a snapshot of a range for inspection/testing.
    pub fn read_range(&self, start: usize, len: usize) -> &[u16] {
        let end = (start + len).min(MEM_SIZE);
        &self.data[start..end]
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_store() {
        let mut mem = Memory::new();
        mem.store(100, 0xABCD).unwrap();
        assert_eq!(mem.load(100).unwrap(), 0xABCD);
    }

    #[test]
    fn out_of_bounds() {
        let mem = Memory::new();
        assert!(mem.load(MEM_SIZE as u16).is_err());
    }

    #[test]
    fn push_pop() {
        let mut mem = Memory::new();
        mem.push(10).unwrap();
        mem.push(20).unwrap();
        assert_eq!(mem.pop().unwrap(), 20);
        assert_eq!(mem.pop().unwrap(), 10);
    }

    #[test]
    fn stack_underflow() {
        let mut mem = Memory::new();
        assert!(mem.pop().is_err());
    }
}
