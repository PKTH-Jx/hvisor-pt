//! Memory related structs and functions.
use vstd::prelude::*;

use super::addr::{PAddr, VAddr};

verus! {

/// Represents a physical memory frame (Page or Block).
pub struct Frame {
    /// The base address of the frame.
    pub base: PAddr,
    /// The size of the frame in bytes.
    pub size: FrameSize,
    /// The attributes of the frame.
    pub attr: FrameAttr,
}

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
    /// Convert to u64.
    pub open spec fn as_u64(self) -> u64 {
        match self {
            FrameSize::Size4K => 0x1000,
            FrameSize::Size2M => 0x200000,
            FrameSize::Size1G => 0x40000000,
        }
    }

    /// Convert to nat.
    pub open spec fn as_nat(self) -> nat {
        self.as_u64() as nat
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

    // /// Allocate a new physical frame.
    // pub open spec fn alloc(&mut self, size: FrameSize) -> (frame: Frame) {
    //     // TODO: allocate a new frame
    //     Frame {
    //         base: PAddr(0),
    //         size: size,
    //         attr: FrameAttr {
    //             readable: true,
    //             writable: true,
    //             executable: true,
    //             user_accessible: true,
    //         },
    //     }
    // }
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

/// Memory read operation and result.
pub struct ReadOp {
    /// Virtual address.
    pub vaddr: VAddr,
    /// Physical address.
    pub paddr: PAddr,
    /// Involved page mapping.
    pub mapping: Option<(VAddr, Frame)>,
    /// Read result.
    pub result: Result<nat, ()>,
}

/// Memory write operation and result.
pub struct WriteOp {
    /// Virtual address.
    pub vaddr: VAddr,
    /// Physical address.
    pub paddr: PAddr,
    /// Value to write.
    pub value: nat,
    /// Involved page mapping.
    pub mapping: Option<(VAddr, Frame)>,
    /// Write result.
    pub result: Result<(), ()>,
}

/// Virtual page map operation and result.
pub struct MapOp {
    /// Virtual page base address.
    pub vaddr: VAddr,
    /// Frame to map.
    pub frame: Frame,
    /// Mapping result.
    pub result: Result<(), ()>,
}

/// Virtual page unmap operation and result.
pub struct UnmapOp {
    /// Virtual page base address.
    pub vaddr: VAddr,
    /// Unmapping result.
    pub result: Result<(), ()>,
}

/// Page table query operation and result.
pub struct QueryOp {
    /// Virtual page address.
    pub vaddr: VAddr,
    /// Query result.
    pub result: Result<(VAddr, Frame), ()>,
}

} // verus!
