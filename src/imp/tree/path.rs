//! The visit path of the abstract page table tree.
use vstd::prelude::*;

use crate::spec::{addr::VAddr, arch::PTArch};

verus! {

/// Represents a path from a node to an entry in the page table tree.
///
/// The path is represented as a sequence of indices, where each index corresponds to
/// an entry in the page table node at a particular level of the hierarchy.
pub struct PTTreePath(pub Seq<nat>);

impl PTTreePath {
    /// Path length.
    pub open spec fn len(self) -> nat {
        self.0.len()
    }

    /// Pop head and return the remaining path.
    pub open spec fn step(self) -> (nat, Self)
        recommends
            self.len() > 0,
    {
        (self.0[0], Self(self.0.skip(1)))
    }

    /// Trim the path to the given length.
    pub open spec fn trim(self, len: nat) -> Self
        recommends
            len <= self.len(),
    {
        Self(self.0.take(len as int))
    }

    /// Check if path is valid.
    pub open spec fn valid(self, arch: PTArch, start_level: nat) -> bool
        recommends
            arch.valid(),
    {
        &&& self.len() > 0
        &&& self.len() + start_level <= arch.level_count()
        &&& forall|i: int|
            0 <= i < self.len() ==> self.0[i] < arch.entry_count(i as nat + start_level)
    }

    /// If `self` has a non-empty prefix `p`.
    pub open spec fn has_prefix(self, p: Self) -> bool {
        &&& 0 < p.len() <= self.len()
        &&& forall|i: int| 0 <= i < p.len() ==> self.0[i] == p.0[i]
    }

    /// Get the first position at which two paths differ.
    pub open spec fn first_diff_idx(a: Self, b: Self) -> int
        recommends
            a.len() > 0,
            b.len() > 0,
            !a.has_prefix(b),
            !b.has_prefix(a),
    {
        choose|i: int|
            0 <= i < a.len() && i < b.len() && a.0[i] != b.0[i] && forall|j: int|
                0 <= j < i ==> a.0[j] == b.0[j]
    }

    /// Get a `Self` from a virtual address, used to query the page table from root.
    ///
    /// The last query level of the returned path is `level`, and the path length is `level + 1`.
    pub open spec fn from_vaddr(vaddr: VAddr, arch: PTArch, level: nat) -> Self
        recommends
            arch.valid(),
            level < arch.level_count(),
    {
        Self(Seq::new(level + 1, |i: int| arch.pte_index_of_va(vaddr, i as nat)))
    }

    /// Calculate the virtual address corresponding to the path from root.
    pub open spec fn to_vaddr(self, arch: PTArch) -> VAddr
        recommends
            arch.valid(),
            self.valid(arch, 0),
    {
        let parts: Seq<nat> = Seq::new(
            self.len(),
            |i: int| self.0[i] * arch.frame_size(i as nat).as_nat(),
        );
        VAddr(parts.fold_left(0, |sum: nat, part| sum + part))
    }

    /// Lemma. Two paths are equal if they have the same first element and the same tail.
    pub proof fn lemma_eq_step(self, other: Self)
        requires
            self.len() > 0,
            other.len() > 0,
            self.step() == other.step(),
        ensures
            self == other,
    {
        let (idx1, remain1) = self.step();
        let (idx2, remain2) = other.step();
        assert(remain1.len() == self.len() - 1);
        assert forall|i| 0 <= i < self.len() implies self.0[i] == other.0[i] by {
            if i == 0 {
                assert(idx1 == idx2);
            } else {
                assert(remain1.0[i - 1] == remain2.0[i - 1]);
            }
        }
        assert(self.0 == other.0);
    }

    /// Lemma. A prefix of a valid path is also valid.
    pub proof fn lemma_prefix_valid(self, arch: PTArch, start_level: nat, pref: Self)
        requires
            arch.valid(),
            self.valid(arch, start_level),
            self.has_prefix(pref),
        ensures
            pref.valid(arch, start_level),
    {
    }

    /// Lemma. If a prefix has the same length as the full path, then the two paths are equal.
    pub proof fn lemma_prefix_equals_full(self, pref: Self)
        requires
            self.has_prefix(pref),
            pref.len() == self.len(),
        ensures
            self == pref,
    {
        assert(self.0 == pref.0);
    }

    /// Lemma. Existence of the first differing index between two distinct paths.
    pub proof fn lemma_first_diff_idx_exists(a: Self, b: Self)
        requires
            a.len() > 0,
            b.len() > 0,
            !a.has_prefix(b),
            !b.has_prefix(a),
        ensures
            exists|i: int|
                0 <= i < a.len() && i < b.len() && a.0[i] != b.0[i] && forall|j: int|
                    0 <= j < i ==> a.0[j] == b.0[j],
    {
        assert(exists|i: int| 0 <= i < a.len() && i < b.len() && a.0[i] != b.0[i]);
        let i = choose|i: int| 0 <= i < a.len() && i < b.len() && a.0[i] != b.0[i];
        Self::lemma_first_diff_idx_exists_recursive(a, b, i);
    }

    /// Helper lemma to prove `lemma_first_diff_idx_exists` by induction.
    proof fn lemma_first_diff_idx_exists_recursive(a: Self, b: Self, i: int)
        requires
            a.len() > 0,
            b.len() > 0,
            !a.has_prefix(b),
            !b.has_prefix(a),
            0 <= i < a.len() && i < b.len() && a.0[i] != b.0[i],
        ensures
            exists|j: int|
                0 <= j <= i && a.0[j] != b.0[j] && forall|k: int| 0 <= k < j ==> a.0[k] == b.0[k],
        decreases i,
    {
        if exists|j: int| 0 <= j < i && a.0[j] != b.0[j] {
            let j = choose|j: int| 0 <= j < i && a.0[j] != b.0[j];
            Self::lemma_first_diff_idx_exists_recursive(a, b, j);
        } else {
            assert(forall|k: int| 0 <= k < i ==> a.0[k] == b.0[k]);
        }
    }

    /// Lemma. `from_vaddr` produces a valid path rooted at level 0.
    pub proof fn lemma_from_vaddr_yields_valid_path(vaddr: VAddr, arch: PTArch, level: nat)
        by (nonlinear_arith)
        requires
            level < arch.level_count(),
            arch.valid(),
        ensures
            Self::from_vaddr(vaddr, arch, level).valid(arch, 0),
    {
        let path = Self::from_vaddr(vaddr, arch, level);
        assert forall|i: int| 0 <= i < path.len() implies path.0[i] < arch.entry_count(
            i as nat,
        ) by {
            // TODO: Verus cannot imply (a % b) < b
            // See: https://verus-lang.github.io/verus/guide/nonlinear.html
            assume(arch.pte_index_of_va(vaddr, i as nat) < arch.entry_count(i as nat))
        }
    }

    /// Lemma. The address computed by `to_vaddr` is aligned to the frame size of the last level.
    pub proof fn lemma_to_vaddr_frame_alignment(self, arch: PTArch)
        by (nonlinear_arith)
        requires
            arch.valid(),
            self.valid(arch, 0),
        ensures
            self.to_vaddr(arch).aligned(arch.frame_size((self.len() - 1) as nat).as_nat()),
    {
        let parts: Seq<nat> = Seq::new(
            self.len(),
            |i: int| self.0[i] * arch.frame_size(i as nat).as_nat(),
        );
        // TODO: This is true by arch.valid(). Recursive proof is needed.
        assume(forall|i|
            0 <= i < self.len() ==> #[trigger] arch.frame_size(i).as_nat() % arch.frame_size(
                (self.len() - 1) as nat,
            ).as_nat() == 0);
        assert(forall|i|
            0 <= i < self.len() ==> parts[i] % arch.frame_size((self.len() - 1) as nat).as_nat()
                == 0);
        let sum = parts.fold_left(0nat, |sum: nat, part| sum + part);
        // TODO: All parts align to the frame size of the last level, prove that sum does too.
        assume(sum % arch.frame_size((self.len() - 1) as nat).as_nat() == 0);
    }

    /// Lemma. If `path` has a prefix `pref`, then `path.to_vaddr()` has a lower bound.
    pub proof fn lemma_to_vaddr_lower_bound(arch: PTArch, path: Self, pref: Self)
        requires
            arch.valid(),
            path.valid(arch, 0),
            path.has_prefix(pref),
        ensures
            pref.to_vaddr(arch).0 <= path.to_vaddr(arch).0,
        decreases path.len(),
    {
        if path.len() <= pref.len() {
            // `pref` equals `path`
            path.lemma_prefix_equals_full(pref);
            assert(path.to_vaddr(arch).0 == pref.to_vaddr(arch).0);
        } else {
            // `pref2` is the longest prefix of `path` and not equal to `path`
            let pref2 = path.trim((path.len() - 1) as nat);
            let parts = Seq::new(
                path.len(),
                |i: int| path.0[i] * arch.frame_size(i as nat).as_nat(),
            );
            let pref2_parts = Seq::new(
                pref2.len(),
                |i: int| pref2.0[i] * arch.frame_size(i as nat).as_nat(),
            );
            assert(parts.take(pref2.len() as int) == pref2_parts);

            // Decompose the sum as "pref2 parts" + "remaining part"
            assert(parts.fold_left(0, |sum: nat, part| sum + part) == pref2_parts.fold_left(
                0,
                |sum: nat, part| sum + part,
            ) + path.0[path.len() - 1] * arch.frame_size((path.len() - 1) as nat).as_nat());
            assert(path.to_vaddr(arch).0 >= pref2.to_vaddr(arch).0);

            // Recursive proof for `pref2` and its prefix `pref`
            assert(pref2.has_prefix(pref));
            Self::lemma_to_vaddr_lower_bound(arch, pref2, pref);
        }
    }

    /// Lemma. If `path` has a prefix `pref`, then `path.to_vaddr()` has an upper bound.
    pub proof fn lemma_to_vaddr_upper_bound(arch: PTArch, path: Self, pref: Self)
        by (nonlinear_arith)
        requires
            arch.valid(),
            path.valid(arch, 0),
            path.has_prefix(pref),
        ensures
            path.to_vaddr(arch).0 <= pref.to_vaddr(arch).0 + arch.frame_size(
                (pref.len() - 1) as nat,
            ).as_nat() - arch.frame_size((path.len() - 1) as nat).as_nat(),
        decreases path.len(),
    {
        if path.len() <= pref.len() {
            // `pref` equals `path`
            path.lemma_prefix_equals_full(pref);
            assert(path.to_vaddr(arch).0 == pref.to_vaddr(arch).0);
        } else {
            // `pref2` is the longest prefix of `path` and not equal to `path`
            let pref2 = path.trim((path.len() - 1) as nat);
            let parts = Seq::new(
                path.len(),
                |i: int| path.0[i] * arch.frame_size(i as nat).as_nat(),
            );
            let pref2_parts = Seq::new(
                pref2.len(),
                |i: int| pref2.0[i] * arch.frame_size(i as nat).as_nat(),
            );
            assert(parts.take(pref2.len() as int) == pref2_parts);

            // Decompose "pref2_parts" as "pref parts" + "remaining part"
            let remain = path.0[path.len() - 1] * arch.frame_size((path.len() - 1) as nat).as_nat();
            assert(parts.fold_left(0, |sum: nat, part| sum + part) == pref2_parts.fold_left(
                0,
                |sum: nat, part| sum + part,
            ) + remain);

            // The remaining part has an upper bound
            assert(path.0[path.len() - 1] <= arch.entry_count((path.len() - 1) as nat) - 1);
            assert(remain <= arch.frame_size((path.len() - 1) as nat).as_nat() * arch.entry_count(
                (path.len() - 1) as nat,
            ) - arch.frame_size((path.len() - 1) as nat).as_nat());
            assert(remain <= arch.frame_size((pref2.len() - 1) as nat).as_nat() - arch.frame_size(
                (path.len() - 1) as nat,
            ).as_nat());

            assert(path.to_vaddr(arch).0 <= pref2.to_vaddr(arch).0 + arch.frame_size(
                (pref2.len() - 1) as nat,
            ).as_nat() - arch.frame_size((path.len() - 1) as nat).as_nat());

            // Recursive proof for `pref2` and its prefix `pref`
            assert(pref2.has_prefix(pref));
            Self::lemma_to_vaddr_upper_bound(arch, pref2, pref);
        }
    }

    /// Lemma. If `a` and `b` are not a prefix of each other, then the order of their virtual
    /// addresses is the same as the order of their path indices.
    pub proof fn lemma_path_order_implies_vaddr_order(arch: PTArch, a: Self, b: Self)
        by (nonlinear_arith)
        requires
            arch.valid(),
            a.valid(arch, 0),
            b.valid(arch, 0),
            !a.has_prefix(b),
            !b.has_prefix(a),
            a.0[Self::first_diff_idx(a, b)] < b.0[Self::first_diff_idx(a, b)],
        ensures
            a.to_vaddr(arch).0 + arch.frame_size((a.len() - 1) as nat).as_nat() <= b.to_vaddr(
                arch,
            ).0,
    {
        // Trim the paths at the first differing index
        Self::lemma_first_diff_idx_exists(a, b);
        let diff_idx = Self::first_diff_idx(a, b);
        let pref_a = a.trim((diff_idx + 1) as nat);
        let pref_b = b.trim((diff_idx + 1) as nat);

        // Bound the full paths by their prefixes
        Self::lemma_to_vaddr_upper_bound(arch, a, pref_a);
        Self::lemma_to_vaddr_lower_bound(arch, b, pref_b);
        assert(a.to_vaddr(arch).0 + arch.frame_size((a.len() - 1) as nat).as_nat()
            <= pref_a.to_vaddr(arch).0 + arch.frame_size((pref_a.len() - 1) as nat).as_nat());
        assert(pref_b.to_vaddr(arch).0 <= b.to_vaddr(arch).0);

        // `common` is the same part shared by `pref_a` and `pref_b`
        assert(pref_a.trim(diff_idx as nat).0 == pref_b.trim(diff_idx as nat).0);
        let common = pref_a.trim(diff_idx as nat);

        // Show `common_parts` is equally added when computing vaddr
        let common_parts = Seq::new(
            common.len(),
            |i: int| common.0[i] * arch.frame_size(i as nat).as_nat(),
        );
        let pref_a_parts = Seq::new(
            pref_a.len(),
            |i: int| pref_a.0[i] * arch.frame_size(i as nat).as_nat(),
        );
        let pref_b_parts = Seq::new(
            pref_b.len(),
            |i: int| pref_b.0[i] * arch.frame_size(i as nat).as_nat(),
        );
        let fsize = arch.frame_size(diff_idx as nat).as_nat();

        assert(pref_a_parts.take(diff_idx) == common_parts);
        assert(pref_a_parts.fold_left(0, |sum: nat, part| sum + part) == common_parts.fold_left(
            0nat,
            |sum: nat, part| sum + part,
        ) + pref_a.0[diff_idx] * fsize);
        assert(pref_b_parts.take(diff_idx) == common_parts);
        assert(pref_b_parts.fold_left(0, |sum: nat, part| sum + part) == common_parts.fold_left(
            0nat,
            |sum: nat, part| sum + part,
        ) + pref_b.0[diff_idx] * fsize);

        // Decompose the sum as "common parts" + "difference part"
        assert(pref_a.to_vaddr(arch).0 == common.to_vaddr(arch).0 + pref_a.0[diff_idx] * fsize);
        assert(pref_b.to_vaddr(arch).0 == common.to_vaddr(arch).0 + pref_b.0[diff_idx] * fsize);

        // Calculate the minimum difference between `pref_a.to_vaddr()` and `pref_b.to_vaddr()`
        assert(pref_b.to_vaddr(arch).0 - pref_a.to_vaddr(arch).0 == (pref_b.0[diff_idx]
            - pref_a.0[diff_idx]) * fsize);
        assert(pref_b.0[diff_idx] - pref_a.0[diff_idx] >= 1);
        assert(pref_b.to_vaddr(arch).0 - pref_a.to_vaddr(arch).0 >= fsize);

        // Prove the bounded inequality
        assert(a.to_vaddr(arch).0 + arch.frame_size((a.len() - 1) as nat).as_nat() <= b.to_vaddr(
            arch,
        ).0);
    }

    /// Lemma. If `a` and `b` are not a prefix of each other, then `a.vaddr() != b.vaddr()`.
    pub proof fn lemma_nonprefix_implies_vaddr_inequality(arch: PTArch, a: Self, b: Self)
        requires
            arch.valid(),
            a.valid(arch, 0),
            b.valid(arch, 0),
            !a.has_prefix(b),
            !b.has_prefix(a),
        ensures
            a.to_vaddr(arch) != b.to_vaddr(arch),
    {
        Self::lemma_first_diff_idx_exists(a, b);
        let diff_idx = Self::first_diff_idx(a, b);
        if a.0[diff_idx] < b.0[diff_idx] {
            Self::lemma_path_order_implies_vaddr_order(arch, a, b);
        } else {
            Self::lemma_path_order_implies_vaddr_order(arch, b, a);
        }
    }

    // Lemma. `to_vaddr` is the inverse of `from_vaddr`
    pub proof fn lemma_to_vaddr_is_inverse_of_from_vaddr(arch: PTArch, vaddr: VAddr, path: Self)
        requires
            arch.valid(),
            path.valid(arch, 0),
            vaddr.aligned(arch.frame_size((path.len() - 1) as nat).as_nat()),
            path == Self::from_vaddr(vaddr, arch, (path.len() - 1) as nat),
        ensures
            path.to_vaddr(arch) == vaddr,
    {
        let parts: Seq<nat> = Seq::new(
            path.len(),
            |i: int| path.0[i] * arch.frame_size(i as nat).as_nat(),
        );
        assert(forall|i|
            0 <= i < path.len() ==> parts[i] == arch.pte_index_of_va(vaddr, i as nat)
                * arch.frame_size(i as nat).as_nat());
        // TODO consider add a lemma to `PTArch`
        assume(false);
    }
}

} // verus!
