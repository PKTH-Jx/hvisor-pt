//! High-level state machine & high-level specifications.
//!
//! This is the high-level abstraction of the memory management module, which gives
//! a completely abstract view of the memory state with all implementation details removed.
//!
//! To prove an implementation’s correctness we need to ask what we mean
//! by correctness. The application specification is a state machine encoding our
//! answer to that question.
//!
//! This specification represents the proof target. Our implementation running
//! in the intended environment should refine it. This is demonstrated in part
//! by the proof that the low-level state machine refines this specification.
use vstd::prelude::*;

use super::{
    addr::{PAddr, VAddr, VIdx, WORD_SIZE},
    frame::{Frame, FrameSize},
};

verus! {

/// High level (abstract) memory state.
pub struct HighLevelState {
    /// 8-byte-indexed virtual memory.
    ///
    /// We use index rather than address. Addresses that are not aligned to 8-byte boundaries
    /// should not be used to access a value, while indexes don't face this issue.
    pub mem: Map<VIdx, u64>,
    /// Mappings from virtual address to physical frames.
    ///
    /// The key must be the base address of a virtual page i.e. virtual range [`key`, `key + frame.size`)
    /// must be mapped to the same physical frame.
    pub mappings: Map<VAddr, Frame>,
    /// Constants.
    pub constants: HighLevelConstants,
}

/// High-level (abstract) memory state constants.
pub struct HighLevelConstants {
    /// Physical memory size (in bytes).
    pub pmem_size: nat,
}

/// State transition specifications.
impl HighLevelState {
    /// Init state. Empty memory and no mappings.
    pub open spec fn init(self) -> bool {
        &&& self.mem === Map::empty()
        &&& self.mappings === Map::empty()
    }

    /// State transition - Read.
    pub open spec fn read(s1: Self, s2: Self, vaddr: VAddr, res: Result<u64, ()>) -> bool {
        &&& vaddr.aligned(
            WORD_SIZE,
        )
        // Memory and mappings should not be updated
        &&& s1.mappings === s2.mappings
        &&& s1.mem === s2.mem
        // Check mapping
        &&& if s1.has_mapping_for(vaddr) {
            let (base, frame) = s1.mapping_for(vaddr);
            // Check frame attributes
            if vaddr.map(base, frame.base).idx().0 < s1.constants.pmem_size && frame.attr.readable
                && frame.attr.user_accessible {
                &&& res is Ok
                // The value should be the value in the memory at `vidx`
                &&& res.unwrap() === s1.mem[vaddr.idx()]
            } else {
                res is Err
            }
        } else {
            res is Err
        }
    }

    /// State transition - write.
    pub open spec fn write(
        s1: Self,
        s2: Self,
        vaddr: VAddr,
        value: u64,
        res: Result<(), ()>,
    ) -> bool {
        &&& vaddr.aligned(WORD_SIZE)
        // Mappings should not be updated
        &&& s1.mappings === s2.mappings
        // Check mapping
        &&& if s1.has_mapping_for(vaddr) {
            let (base, frame) = s1.mapping_for(vaddr);
            // Check frame attributes
            if vaddr.map(base, frame.base).idx().0 < s1.constants.pmem_size && frame.attr.writable
                && frame.attr.user_accessible {
                &&& res is Ok
                // Memory should be updated at `vidx` with `value`
                &&& s2.mem === s1.mem.insert(vaddr.idx(), value)
            } else {
                &&& res is Err
                // Memory should not be updated
                &&& s1.mem === s2.mem
            }
        } else {
            // The result should be `Err`
            &&& res is Err
            // Memory should not be updated
            &&& s1.mem === s2.mem
        }
    }

    /// State transtion - Map a virtual address to a frame.
    pub open spec fn map(
        s1: Self,
        s2: Self,
        vaddr: VAddr,
        frame: Frame,
        res: Result<(), ()>,
    ) -> bool {
        // Base vaddr should align to frame size
        &&& vaddr.aligned(
            frame.size.as_nat(),
        )
        // Base paddr should align to frame size
        &&& frame.base.aligned(
            frame.size.as_nat(),
        )
        // Frame should not overlap with existing pmem
        &&& !s1.overlaps_pmem(frame)
        // Check vmem overlapping
        &&& if s1.overlaps_vmem(vaddr, frame) {
            // Mapping fails
            &&& res is Err
            // Memory and mappings should not be updated
            &&& s1.mem === s2.mem
            &&& s1.mappings === s2.mappings
        } else {
            // Mapping succeeds
            &&& res is Ok
            // Update mappings
            &&& s1.mappings.insert(vaddr, frame)
                === s2.mappings
            // Memory domain should be updated
            &&& s2.mem.dom() === s2.mem_domain_covered_by_mappings()
        }
    }

    /// State transtion - Unmap a virtual address.
    pub open spec fn unmap(s1: Self, s2: Self, vaddr: VAddr, res: Result<(), ()>) -> bool {
        // Base vaddr should align to some valid frame size
        &&& {
            ||| vaddr.aligned(FrameSize::Size4K.as_nat())
            ||| vaddr.aligned(FrameSize::Size2M.as_nat())
            ||| vaddr.aligned(FrameSize::Size1G.as_nat())
        }
        // Check mapping
        &&& if s1.mappings.contains_key(vaddr) {
            // Unmapping succeeds
            &&& res is Ok
            // Update mappings
            &&& s1.mappings.remove(vaddr)
                === s2.mappings
            // Memory domain should be updated
            &&& s2.mem.dom() === s2.mem_domain_covered_by_mappings()
        } else {
            // Unmapping fails
            &&& res is Err
            // Memory and mappings should not be updated
            &&& s1.mem === s2.mem
            &&& s1.mappings === s2.mappings
        }
    }

    /// State transition - Page table query.
    pub open spec fn query(
        s1: Self,
        s2: Self,
        vaddr: VAddr,
        res: Result<(VAddr, Frame), ()>,
    ) -> bool {
        // Memory and mappings should not be updated
        &&& s1.mem === s2.mem
        &&& s1.mappings === s2.mappings
        // Check result
        &&& match res {
            Ok((base, frame)) => {
                // Must contain the mapping
                &&& s1.mappings.contains_pair(base, frame)
                &&& vaddr.within(base, frame.size.as_nat())
            },
            Err(_) => {
                // Should not contain any mapping for vaddr
                !s1.has_mapping_for(vaddr)
            },
        }
    }

    /// State transition - Identity.
    pub open spec fn id(s1: Self, s2: Self) -> bool {
        s1 === s2
    }
}

/// Helper functions.
impl HighLevelState {
    /// Virtual memory domain covered by `self.mappings`.
    pub open spec fn mem_domain_covered_by_mappings(self) -> Set<VIdx> {
        Set::new(
            |vidx: VIdx|
                exists|base: VAddr, frame: Frame|
                    {
                        &&& #[trigger] self.mappings.contains_pair(base, frame)
                        &&& vidx.addr().within(base, frame.size.as_nat())
                    },
        )
    }

    /// If `frame` overlaps with existing physical memory.
    pub open spec fn overlaps_pmem(self, frame: Frame) -> bool {
        exists|frame1: Frame|
            {
                &&& #[trigger] self.mappings.contains_value(frame1)
                &&& PAddr::overlap(
                    frame1.base,
                    frame1.size.as_nat(),
                    frame.base,
                    frame.size.as_nat(),
                )
            }
    }

    /// If mapping `(vaddr, frame)` overlaps with existing virtual memory.
    pub open spec fn overlaps_vmem(self, vaddr: VAddr, frame: Frame) -> bool {
        exists|base: VAddr|
            {
                &&& #[trigger] self.mappings.contains_key(base)
                &&& VAddr::overlap(
                    base,
                    self.mappings[base].size.as_nat(),
                    vaddr,
                    frame.size.as_nat(),
                )
            }
    }

    /// If there exists a mapping for `vaddr`.
    pub open spec fn has_mapping_for(self, vaddr: VAddr) -> bool {
        exists|base: VAddr, frame: Frame|
            {
                &&& #[trigger] self.mappings.contains_pair(base, frame)
                &&& vaddr.within(base, frame.size.as_nat())
            }
    }

    /// Get the mapping for `vaddr`.
    pub open spec fn mapping_for(self, vaddr: VAddr) -> (VAddr, Frame)
        recommends
            self.has_mapping_for(vaddr),
    {
        choose|base: VAddr, frame: Frame|
            {
                &&& #[trigger] self.mappings.contains_pair(base, frame)
                &&& vaddr.within(base, frame.size.as_nat())
            }
    }
}

} // verus!
