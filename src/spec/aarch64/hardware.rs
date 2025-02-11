//! Hardware abstract state and state machine.
use vstd::prelude::*;

use super::mem::{Frame, MemoryState};

verus! {

/// Hardware memory state transition specifications
impl MemoryState {
    /// State transition - Common memory read & write operation.
    ///
    /// The pre-state `s1` and post-state `s2` must satisfy the specification
    /// after common memory read & write operation.
    pub open spec fn mem_read_write(
        s1: Self,
        s2: Self,
        vaddr: nat,
        paddr: nat,
        write: bool,
    ) -> bool {
        // TODO
        true
    }

    /// State transition - Page table memory operation.
    ///
    /// The pre-state `s1` and post-state `s2` must satisfy the specification
    /// after page table memory operation.
    pub open spec fn pt_mem_op(s1: Self, s2: Self) -> bool {
        // TODO
        true
    }

    /// State transition - TLB fill operation.
    ///
    /// The pre-state `s1` and post-state `s2` must satisfy the specification
    /// after TLB fill operation.
    pub open spec fn tlb_fill(s1: Self, s2: Self, vaddr: nat, frame: Frame) -> bool {
        // TODO
        true
    }

    /// State transition - TLB eviction operation.
    ///
    /// The pre-state `s1` and post-state `s2` must satisfy the specification
    /// after TLB eviction operation.
    pub open spec fn tlb_evict(s1: Self, s2: Self, vaddr: nat) -> bool {
        // TODO
        true
    }
}

} // verus!
