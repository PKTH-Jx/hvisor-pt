//! Defination of memory management structures and functions.
use vstd::prelude::*;

verus! {

/// Page & Block size supported by VMSA-v8.
pub enum FrameSize {
    /// 4 KiB
    Size4K,
    /// 2 MiB
    Size2M,
    /// 1 GiB
    Size1G,
}

impl FrameSize {
    pub open spec fn as_u64(self) -> u64 {
        match self {
            FrameSize::Size4K => 0x1000,
            FrameSize::Size2M => 0x200000,
            FrameSize::Size1G => 0x40000000,
        }
    }
}

/// Frame attributes. Defination consistent with `hvisor::memory::MemFlags`.
#[derive(PartialEq, Eq)]
pub struct FrameAttr {
    /// Whether the memory is readable.
    pub readable: bool,
    /// Whether the memory is writable.
    pub writable: bool,
    /// Whether the memory is executable.
    pub executable: bool,
    /// Whether the memory is user accessible.
    pub user_accessible: bool,
}

/// Represents a physical memory frame (Page or Block).
pub struct Frame {
    /// The base address of the frame.
    pub base: u64,
    /// The size of the frame in bytes.
    pub size: FrameSize,
    /// The attributes of the frame.
    pub attr: FrameAttr,
}

/// Memory where page table is stored.
pub struct PageTableMem {
    // TODO: fields
}

impl PageTableMem {
    pub fn new() -> Self {
        Self {  }
    }

    /// Physical address of the root page table.
    pub open spec fn root(self) -> u64 {
        0
    }

    /// Read value at physical address `base + idx * WORD_SIZE`
    pub fn read(&self, base: u64, idx: u64) -> (res: u64) {
        0
    }

    /// Write `value` to physical address `base + idx * WORD_SIZE`
    pub fn write(&mut self, base: u64, idx: u64, value: u64) {
        // TODO: write to memory
    }

    /// Allocate a new physical frame.
    pub fn alloc(&mut self, size: FrameSize) -> (frame: Frame) {
        // TODO: allocate a new frame
        Frame {
            base: 0,
            size: size,
            attr: FrameAttr {
                readable: true,
                writable: true,
                executable: true,
                user_accessible: true,
            },
        }
    }

    /// Deallocate a physical frame.
    pub fn dealloc(&mut self, frame: Frame) {
        // TODO: deallocate a frame
    }

    /// Specification of read operation.
    pub open spec fn spec_read(&self, addr: u64) -> u64 {
        // TODO: read from memory
        0
    }
}

} // verus!
