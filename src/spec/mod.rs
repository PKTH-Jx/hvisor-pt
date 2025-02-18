use vstd::prelude::*;

mod hardware;
mod hl;
mod mem;
mod os;
mod pt;
mod s1pt;
mod s2pt;

verus! {

/// Convert `nat` to `u64`.
pub open spec fn nat_to_u64(v: nat) -> u64
    recommends
        v <= u64::MAX,
{
    v as u64
}

/// If region (base1, size1) and region (base2, size2) overlap.
pub open spec(checked) fn overlap(base1: nat, size1: nat, base2: nat, size2: nat) -> bool {
    if base1 <= base2 {
        base2 < base1 + size1
    } else {
        base1 < base2 + size2
    }
}

/// Representing virtual address.
#[derive(Clone, Copy)]
pub struct VAddr(pub nat);

impl VAddr {
    /// Convert to word index.
    pub open spec fn word_idx(self) -> VWordIdx {
        VWordIdx(self.0 / 8)
    }

    /// If addr is aligned to `size` bytes.
    pub open spec fn aligned(self, size: nat) -> bool {
        self.0 % size == 0
    }

    /// If addr is in range `[lb, ub)`.
    pub open spec fn between(self, lb: Self, ub: Self) -> bool {
        lb.0 <= self.0 < ub.0
    }

    /// Offset by `offset` bytes.
    pub open spec fn offset(self, offset: nat) -> VAddr {
        VAddr(self.0 + offset)
    }

    /// If virtual region (base1, size1) and virtual region (base2, size2) overlap.
    pub open spec fn overlap(base1: Self, size1: nat, base2: Self, size2: nat) -> bool {
        overlap(base1.0, size1, base2.0, size2)
    }

    /// If virtual page base `vbase` maps to physical page base `pbase`, calc the physical
    /// address that `self` maps to.
    pub open spec fn translate(self, vbase: Self, pbase: PAddr) -> PAddr
        recommends
            self.0 >= vbase.0,
    {
        PAddr((self.0 - vbase.0) as nat + pbase.0)
    }
}

/// Representing physical address.
#[derive(Clone, Copy)]
pub struct PAddr(pub nat);

impl PAddr {
    /// Convert to word index.
    pub open spec fn word_idx(self) -> PWordIdx {
        PWordIdx(self.0 / 8)
    }

    /// If addr is aligned to `size` bytes.
    pub open spec fn aligned(self, size: nat) -> bool {
        self.0 % size == 0
    }

    /// If physical region (base1, size1) and physical region (base2, size2) overlap.
    pub open spec fn overlap(base1: Self, size1: nat, base2: Self, size2: nat) -> bool {
        overlap(base1.0, size1, base2.0, size2)
    }
}

/// Index used to access virtual memory by word.
pub struct VWordIdx(pub nat);

impl VWordIdx {
    /// Convert to virtual address.
    pub open spec fn addr(self) -> VAddr {
        VAddr(self.0 * 8)
    }

    /// Convert to int.
    pub open spec fn as_int(self) -> int {
        self.0 as int
    }
}

/// Index used to access physical memory by word.
pub struct PWordIdx(pub nat);

impl PWordIdx {
    /// Convert to physical address.
    pub open spec fn addr(self) -> PAddr {
        PAddr(self.0 * 8)
    }

    /// Convert to int.
    pub open spec fn as_int(self) -> int {
        self.0 as int
    }
}

} // verus!
