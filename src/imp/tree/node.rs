//! The node of the abstract page table tree, providing basic visition and manipulation algorithms.
use vstd::prelude::*;

use super::path::PTTreePath;
use crate::{
    common::{addr::VAddr, frame::Frame, PagingResult},
    imp::lemmas::lemma_map_eq_pair,
    spec::page_table::PTConstants,
};

verus! {

// Use path related lemmas.
broadcast use super::path::group_pt_tree_path_lemmas;

/// Represents a node in the page table tree, which can be either an intermediate node
/// or a leaf node mapping to a physical frame.
pub struct PTTreeNode {
    /// Page table config constants.
    pub constants: PTConstants,
    /// The level of the node in the page table hierarchy.
    pub level: nat,
    /// The entries of the node, which can be either sub-nodes, frames, or empty entries.
    pub entries: Seq<NodeEntry>,
}

/// Represents an entry in the page table node, which can be a sub-node, a physical frame,
/// or an empty entry.
pub enum NodeEntry {
    /// A sub-node in the page table, representing an intermediate level of the page table hierarchy.
    Node(PTTreeNode),
    /// A physical frame mapped by the node, representing a leaf node in the page table tree.
    Frame(Frame),
    /// An empty entry in the page table, indicating that the corresponding virtual address range
    /// is not currently mapped or allocated.
    Empty,
}

impl PTTreeNode {
    /// If a node entry at the specified level is valid under the given configuration.
    pub open spec fn is_entry_valid(entry: NodeEntry, level: nat, constants: PTConstants) -> bool
        recommends
            level < constants.arch.level_count(),
    {
        match entry {
            NodeEntry::Node(node) => if level < constants.arch.level_count() - 1 {
                &&& node.level == level
                    + 1
                // All nodes share the same configuration.
                &&& node.constants == constants
            } else {
                false
            },
            NodeEntry::Frame(frame) => {
                &&& frame.size == constants.arch.frame_size(level)
                &&& frame.base.aligned(frame.size.as_nat())
                &&& frame.base.0 >= constants.pmem_lb.0
                &&& frame.base.0 + frame.size.as_nat() <= constants.pmem_ub.0
            },
            NodeEntry::Empty => true,
        }
    }

    /// Invariants. Recursively checks the invariants of the node and its sub-nodes.
    ///
    /// This ensures a sub-tree is well-formed, and all mappings are valid and aligned.
    pub open spec fn invariants(self) -> bool
        decreases self.constants.arch.level_count() - self.level,
    {
        &&& self.constants.arch.valid()
        &&& self.level < self.constants.arch.level_count()
        &&& self.entries.len() == self.constants.arch.entry_count(self.level)
        &&& forall|entry: NodeEntry| #[trigger]
            self.entries.contains(entry) ==> {
                &&& Self::is_entry_valid(entry, self.level, self.constants)
                &&& entry is Node ==> entry->Node_0.invariants()
            }
    }

    /// Creates an empty node.
    pub open spec fn new(config: PTConstants, level: nat) -> Self
        recommends
            level < config.arch.level_count(),
            config.arch.valid(),
    {
        Self {
            constants: config,
            level,
            entries: seq![NodeEntry::Empty; config.arch.entry_count(level)],
        }
    }

    /// Update an entry in the node at the specified index.
    pub open spec fn update(self, index: nat, entry: NodeEntry) -> Self
        recommends
            index < self.entries.len(),
            Self::is_entry_valid(entry, self.level, self.constants),
    {
        Self { entries: self.entries.update(index as int, entry), ..self }
    }

    /// Visit the tree along `path` and return the sequence of entries visited.
    ///
    /// If a reached entry is `Empty` and `path` is not empty, then the visit
    /// terminates early and returns the sequence of entries visited so far.
    pub open spec fn recursive_visit(self, path: PTTreePath) -> Seq<NodeEntry>
        recommends
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        decreases path.len(),
    {
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        if path.len() <= 1 {
            seq![entry]
        } else {
            match entry {
                NodeEntry::Node(node) => seq![entry].add(node.recursive_visit(remain)),
                _ => seq![entry],
            }
        }
    }

    /// Inserts a frame at `path`, creates intermediate nodes if needed.
    ///
    /// Does nothing if target slot is non-empty.
    pub open spec fn recursive_insert(self, path: PTTreePath, frame: Frame) -> (Self, PagingResult)
        recommends
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            Self::is_entry_valid(
                NodeEntry::Frame(frame),
                (self.level + path.len() - 1) as nat,
                self.constants,
            ),
        decreases path.len(),
    {
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        if path.len() <= 1 {
            match entry {
                NodeEntry::Empty => (self.update(idx, NodeEntry::Frame(frame)), Ok(())),
                _ => (self, Err(())),
            }
        } else {
            match entry {
                NodeEntry::Node(node) => {
                    let (node, res) = node.recursive_insert(remain, frame);
                    (self.update(idx, NodeEntry::Node(node)), res)
                },
                NodeEntry::Empty => {
                    let (node, res) = Self::new(self.constants, self.level + 1).recursive_insert(
                        remain,
                        frame,
                    );
                    (self.update(idx, NodeEntry::Node(node)), res)
                },
                _ => (self, Err(())),
            }
        }
    }

    /// Removes a frame at `path` by setting it to `Empty`.
    ///
    /// Does nothing if the entry at `path` is already `Empty`.
    pub open spec fn recursive_remove(self, path: PTTreePath) -> (Self, PagingResult)
        recommends
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        decreases path.len(),
    {
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        if path.len() <= 1 {
            match entry {
                NodeEntry::Frame(_) => (self.update(idx, NodeEntry::Empty), Ok(())),
                _ => (self, Err(())),
            }
        } else {
            match entry {
                NodeEntry::Node(node) => {
                    let (node, res) = node.recursive_remove(remain);
                    (self.update(idx, NodeEntry::Node(node)), res)
                },
                NodeEntry::Frame(frame) => {
                    if remain.is_zero() {
                        (self.update(idx, NodeEntry::Empty), Ok(()))
                    } else {
                        (self, Err(()))
                    }
                },
                _ => (self, Err(())),
            }
        }
    }

    /// Returns the longest prefix of `path` that can reach an entry without early terminating.
    pub open spec fn real_path(self, path: PTTreePath) -> PTTreePath
        recommends
            self.invariants(),
            path.valid(self.constants.arch, self.level),
    {
        path.trim(self.recursive_visit(path).len())
    }

    /// If `recursive_visit(path)` terminates exactly at a frame.
    pub open spec fn is_frame_path(self, path: PTTreePath) -> bool
        recommends
            self.invariants(),
    {
        path.valid(self.constants.arch, self.level) && self.recursive_visit(path).last() is Frame
            && self.recursive_visit(path).len() == path.len()
    }

    /// Collect all paths that terminate at a frame as a map.
    pub open spec fn path_mappings(self) -> Map<PTTreePath, Frame>
        recommends
            self.invariants(),
    {
        Map::new(|path| self.is_frame_path(path), |path| self.recursive_visit(path).last()->Frame_0)
    }

    /// Lemma. `new` implies invariants.
    pub proof fn lemma_new_implies_invariants(config: PTConstants, level: nat)
        requires
            level < config.arch.level_count(),
            config.arch.valid(),
        ensures
            Self::new(config, level).invariants(),
    {
    }

    /// Lemma. `update` preserves invariants.
    pub proof fn lemma_update_preserves_invariants(self, index: nat, entry: NodeEntry)
        requires
            self.invariants(),
            0 <= index < self.entries.len(),
            Self::is_entry_valid(entry, self.level, self.constants),
            entry is Node ==> entry->Node_0.invariants(),
        ensures
            self.update(index, entry).invariants(),
    {
        let new = self.update(index, entry);
        assert forall|entry2: NodeEntry| #[trigger]
            new.entries.contains(entry2) implies Self::is_entry_valid(
            entry2,
            self.level,
            self.constants,
        ) by {
            if entry2 != entry {
                assert(self.entries.contains(entry2));
            }
        }
        assert forall|entry2: NodeEntry| #[trigger]
            new.entries.contains(entry2) implies match entry2 {
            NodeEntry::Node(node) => node.invariants(),
            _ => true,
        } by {
            if entry2 != entry {
                assert(self.entries.contains(entry2));
            }
        }
    }

    /// Lemma. `recursive_insert` preserves invariants.
    pub proof fn lemma_insert_preserves_invariants(self, path: PTTreePath, frame: Frame)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            Self::is_entry_valid(
                NodeEntry::Frame(frame),
                (self.level + path.len() - 1) as nat,
                self.constants,
            ),
        ensures
            self.recursive_insert(path, frame).0.invariants(),
        decreases path.len(),
    {
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        assert(self.entries.contains(entry));
        if path.len() <= 1 {
            // Base case, proved by lemma
            self.lemma_update_preserves_invariants(idx, NodeEntry::Frame(frame));
        } else {
            match entry {
                NodeEntry::Node(node) => {
                    assert(Self::is_entry_valid(entry, self.level, self.constants));
                    assert(node.invariants());
                    // Recursively prove `node.recursive_insert(remain, frame)`
                    node.lemma_insert_preserves_invariants(remain, frame);
                    // `node.recursive_update(remain, frame)` satisfies invariants,
                    // so the updated `self` also satisfy invariants by lemma
                    self.lemma_update_preserves_invariants(
                        idx,
                        NodeEntry::Node(node.recursive_insert(remain, frame).0),
                    );
                },
                NodeEntry::Empty => {
                    let new = PTTreeNode::new(self.constants, self.level + 1);
                    // `new` satisfies invariants by construction
                    assert(new.invariants());
                    // Recursively prove `new.recursive_insert(remain, frame)`
                    new.lemma_insert_preserves_invariants(remain, frame);
                    // `new.recursive_insert(remain, frame)` satisfies invariants,
                    // so the updated `self` also satisfy invariants by lemma
                    self.lemma_update_preserves_invariants(
                        idx,
                        NodeEntry::Node(new.recursive_insert(remain, frame).0),
                    );
                },
                _ => (),
            }
        }
    }

    /// Lemma. `recursive_remove` preserves invariants.
    pub proof fn lemma_remove_preserves_invariants(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            self.recursive_remove(path).0.invariants(),
        decreases path.len(),
    {
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        assert(self.entries.contains(entry));
        if path.len() <= 1 {
            // Base case, proved by lemma
            self.lemma_update_preserves_invariants(idx, NodeEntry::Empty);
        } else {
            match entry {
                NodeEntry::Node(node) => {
                    assert(Self::is_entry_valid(entry, self.level, self.constants));
                    assert(node.invariants());
                    // Recursively prove `node.recursive_remove(remain)`
                    node.lemma_remove_preserves_invariants(remain);
                    // `node.recursive_remove(remain)` satisfies invariants,
                    // so the updated `self` also satisfy invariants by lemma
                    self.lemma_update_preserves_invariants(
                        idx,
                        NodeEntry::Node(node.recursive_remove(remain).0),
                    );
                },
                NodeEntry::Frame(_) => {
                    if remain.is_zero() {
                        self.lemma_update_preserves_invariants(idx, NodeEntry::Empty);
                    }
                },
                _ => (),
            }
        }
    }

    /// Lemma. Length of `recursive_visit(path)` is between 1 and `path.len()`.
    pub proof fn lemma_visit_length_bounds(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            1 <= self.recursive_visit(path).len() <= path.len(),
        decreases path.len(),
    {
        if path.len() <= 1 {
            assert(self.recursive_visit(path).len() == 1);
        } else {
            let (idx, remain) = path.step();
            let entry = self.entries[idx as int];
            assert(self.entries.contains(entry));
            match entry {
                NodeEntry::Node(node) => {
                    node.lemma_visit_length_bounds(remain);
                },
                _ => assert(self.recursive_visit(path).len() == 1),
            }
        }
    }

    /// Lemma. Every entry returned by `recursive_visit` satisfies `is_entry_valid`.
    pub proof fn lemma_visited_entries_satisfy_invariants(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            forall|i: int|
                #![auto]
                0 <= i < self.recursive_visit(path).len() ==> Self::is_entry_valid(
                    self.recursive_visit(path)[i],
                    self.level + i as nat,
                    self.constants,
                ),
        decreases path.len(),
    {
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        assert(self.entries.contains(entry));
        assert(Self::is_entry_valid(entry, self.level, self.constants));
        if path.len() > 1 {
            match entry {
                NodeEntry::Node(node) => {
                    assert(self.recursive_visit(path) === seq![entry].add(
                        node.recursive_visit(remain),
                    ));
                    // Recursively prove `node.recursive_visit(remain)` satisfies the invariants
                    node.lemma_visited_entries_satisfy_invariants(remain);
                },
                _ => (),
            }
        }
    }

    /// Lemma. All visited entries except the final one must be sub-nodes.
    pub proof fn lemma_visited_entry_is_node_except_final(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            forall|i|
                0 <= i < self.recursive_visit(path).len() - 1 ==> self.recursive_visit(
                    path,
                )[i] is Node,
        decreases path.len(),
    {
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        assert(self.entries.contains(entry));
        assert(Self::is_entry_valid(entry, self.level, self.constants));
        if path.len() > 1 {
            match entry {
                NodeEntry::Node(node) => {
                    assert(self.recursive_visit(path) === seq![entry].add(
                        node.recursive_visit(remain),
                    ));
                    // Recursively prove `node.recursive_visit(remain)`
                    node.lemma_visited_entry_is_node_except_final(remain);
                },
                _ => (),
            }
        }
    }

    /// Lemma. If `prefix` is a prefix of `path`, then the visit sequence of `prefix`
    /// is a prefix of that of `path`.
    pub proof fn lemma_visit_preserves_prefix(self, path: PTTreePath, prefix: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            path.has_prefix(prefix),
        ensures
            self.recursive_visit(prefix).len() <= self.recursive_visit(path).len(),
            forall|i|
                0 <= i < self.recursive_visit(prefix).len() ==> self.recursive_visit(prefix)[i]
                    === self.recursive_visit(path)[i],
        decreases path.len(),
    {
        let visited = self.recursive_visit(path);
        let p_visited = self.recursive_visit(prefix);
        let (idx, remain) = path.step();
        let (p_idx, p_remain) = prefix.step();
        assert(p_idx == idx);
        let entry = self.entries[p_idx as int];
        assert(self.entries.contains(entry));

        if prefix.len() > 1 {
            match entry {
                NodeEntry::Node(node) => {
                    assert(visited == seq![entry].add(node.recursive_visit(remain)));
                    assert(p_visited == seq![entry].add(node.recursive_visit(p_remain)));
                    // Recursively prove `node.recursive_visit(remain)`
                    node.lemma_visit_preserves_prefix(remain, p_remain);
                    assert forall|i| 0 < i < p_visited.len() implies p_visited[i] == visited[i] by {
                        assert(p_visited[i] == node.recursive_visit(p_remain)[i - 1]);
                        assert(visited[i] == node.recursive_visit(remain)[i - 1]);
                    }
                },
                _ => (),
            }
        }
    }

    /// Lemma. If `path` is valid, then `real_path(path)` is also valid.
    pub proof fn lemma_real_path_valid(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            self.real_path(path).valid(self.constants.arch, self.level),
    {
        self.lemma_visit_length_bounds(path);
    }

    /// Lemma. The entry sequence visited by `path` and `real_path(path)` are the same.
    pub proof fn lemma_real_path_visits_same_entry(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            self.recursive_visit(self.real_path(path)) == self.recursive_visit(path),
        decreases path.len(),
    {
        let r_path = self.real_path(path);
        let visited = self.recursive_visit(path);
        let r_visited = self.recursive_visit(r_path);

        let (idx, remain) = path.step();
        let (r_idx, r_remain) = r_path.step();
        self.lemma_visit_length_bounds(path);
        self.lemma_visit_length_bounds(r_path);
        assert(idx == r_idx);
        let entry = self.entries[r_idx as int];
        assert(self.entries.contains(entry));

        if r_path.len() <= 1 {
            assert(r_visited == seq![entry]);
            assert(visited == seq![entry]);
        } else {
            match entry {
                NodeEntry::Node(node) => {
                    assert(r_visited == seq![entry].add(node.recursive_visit(r_remain)));
                    assert(visited == seq![entry].add(node.recursive_visit(remain)));
                    // Prove `r_remain` is the real path of `remain`
                    assert(node.real_path(remain).0 == path.0.skip(1).take(visited.len() - 1));
                    assert(r_remain.0 == path.0.skip(1).take(visited.len() - 1));
                    assert(r_remain == node.real_path(remain));
                    // Recursively prove `node.recursive_visit(remain)`
                    node.lemma_real_path_visits_same_entry(remain);
                },
                _ => {
                    assert(r_visited == seq![entry]);
                    assert(visited == seq![entry]);
                },
            }
        }
    }

    /// Lemma. If `self.recursive_visit(path)` reaches a frame, then `self.real_path(path)`
    /// is contained in path mappings.
    pub proof fn lemma_real_path_in_path_mappings(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            self.recursive_visit(path).last() is Frame,
        ensures
            self.path_mappings().contains_key(self.real_path(path)),
    {
        self.lemma_visit_length_bounds(path);
        self.lemma_real_path_valid(path);
        self.lemma_real_path_visits_same_entry(path);
    }

    /// Lemma. All `(path, frame)` mappings have valid size and alignment.
    pub proof fn lemma_path_mappings_valid(self)
        requires
            self.invariants(),
            self.level == 0,
        ensures
            forall|path, frame| #[trigger]
                self.path_mappings().contains_pair(path, frame) ==> {
                    &&& frame.size == self.constants.arch.frame_size((path.len() - 1) as nat)
                    &&& path.to_vaddr(self.constants.arch).aligned(frame.size.as_nat())
                    &&& frame.base.aligned(frame.size.as_nat())
                    &&& frame.base.0 >= self.constants.pmem_lb.0
                    &&& frame.base.0 + frame.size.as_nat() <= self.constants.pmem_ub.0
                },
    {
        assert forall|path, frame| #[trigger]
            self.path_mappings().contains_pair(path, frame) implies {
            &&& frame.size == self.constants.arch.frame_size((path.len() - 1) as nat)
            &&& path.to_vaddr(self.constants.arch).aligned(frame.size.as_nat())
            &&& frame.base.aligned(frame.size.as_nat())
            &&& frame.base.0 >= self.constants.pmem_lb.0
            &&& frame.base.0 + frame.size.as_nat() <= self.constants.pmem_ub.0
        } by {
            // Prove the reached frame satisfy the invariants
            self.lemma_visited_entries_satisfy_invariants(path);
            assert(PTTreeNode::is_entry_valid(
                NodeEntry::Frame(frame),
                (path.len() - 1) as nat,
                self.constants,
            ));
            assert(self.constants.arch.is_valid_frame_size(frame.size));

            // Prove alignment
            assert(frame.base.aligned(frame.size.as_nat()));
            assert(frame.base.0 >= self.constants.pmem_lb.0);
            assert(frame.base.0 + frame.size.as_nat() <= self.constants.pmem_ub.0);
            path.lemma_to_vaddr_frame_alignment(self.constants.arch);
        }
    }

    /// Lemma. Path mappings can not have 2 keys `a` and `b` such that `a` is prefix of `b`.
    pub proof fn lemma_path_mappings_nonprefix(self)
        requires
            self.invariants(),
        ensures
            forall|path1, path2|
                self.path_mappings().contains_key(path1) && self.path_mappings().contains_key(path2)
                    ==> path1 == path2 || !path1.has_prefix(path2),
    {
        assert forall|path1, path2|
            self.path_mappings().contains_key(path1) && self.path_mappings().contains_key(
                path2,
            ) implies path1 == path2 || !path1.has_prefix(path2) by {
            if path1 != path2 {
                if path1.has_prefix(path2) {
                    // Proof by contradiction
                    let visited1 = self.recursive_visit(path1);
                    let visited2 = self.recursive_visit(path2);
                    assert(visited1.len() == path1.len());
                    assert(visited2.len() == path2.len());
                    // Prove `path2.len() - 1` and `path1.len() - 1` are different
                    if path1.len() == path2.len() {
                        path1.lemma_prefix_eq_full(path2);
                    }
                    assert(path2.len() < path1.len());
                    self.lemma_visit_preserves_prefix(path1, path2);
                    assert(visited1[path2.len() - 1] == visited2.last());
                    assert(visited2.last() is Frame);
                    assert(visited1.last() is Frame);
                    // `visited1` cannot have 2 frames (lemma), contradiction
                    self.lemma_visited_entry_is_node_except_final(path1);
                }
            }
        }
    }

    /// Lemma. All `(path, frame)` mappings do not overlap in virtual memory.
    pub proof fn lemma_path_mappings_nonoverlap_in_vmem(self)
        requires
            self.invariants(),
            self.level == 0,
        ensures
            forall|path1, path2, frame1, frame2|
                self.path_mappings().contains_pair(path1, frame1)
                    && self.path_mappings().contains_pair(path2, frame2) ==> path1 == path2
                    || !VAddr::overlap(
                    path1.to_vaddr(self.constants.arch),
                    frame1.size.as_nat(),
                    path2.to_vaddr(self.constants.arch),
                    frame2.size.as_nat(),
                ),
    {
        assert forall|path1, path2, frame1, frame2|
            self.path_mappings().contains_pair(path1, frame1) && self.path_mappings().contains_pair(
                path2,
                frame2,
            ) implies path1 == path2 || !VAddr::overlap(
            path1.to_vaddr(self.constants.arch),
            frame1.size.as_nat(),
            path2.to_vaddr(self.constants.arch),
            frame2.size.as_nat(),
        ) by {
            if path1 != path2 {
                self.lemma_path_mappings_nonprefix();
                assert(!path1.has_prefix(path2));
                assert(!path2.has_prefix(path1));
                PTTreePath::lemma_first_diff_idx_exists(path1, path2);
                let first_diff_idx = PTTreePath::first_diff_idx(path1, path2);
                assert(path1.0[first_diff_idx] != path2.0[first_diff_idx]);

                self.lemma_path_mappings_valid();
                if path1.0[first_diff_idx] < path2.0[first_diff_idx] {
                    PTTreePath::lemma_path_order_implies_vaddr_order(
                        self.constants.arch,
                        path1,
                        path2,
                    );
                    assert(path1.to_vaddr(self.constants.arch).0 + frame1.size.as_nat()
                        <= path2.to_vaddr(self.constants.arch).0);
                } else {
                    PTTreePath::lemma_path_order_implies_vaddr_order(
                        self.constants.arch,
                        path2,
                        path1,
                    );
                    assert(path2.to_vaddr(self.constants.arch).0 + frame2.size.as_nat()
                        <= path1.to_vaddr(self.constants.arch).0);
                }
            }
        }
    }

    /// Lemma. `path_mappings` has at most one path `path` such that `path.to_vaddr() == vbase`.
    pub proof fn lemma_path_mappings_has_at_most_one_path_for_vbase(self, vbase: VAddr)
        requires
            self.invariants(),
            self.level == 0,
        ensures
            forall|path1: PTTreePath, path2: PTTreePath|
                #![auto]
                {
                    &&& self.path_mappings().contains_key(path1)
                    &&& self.path_mappings().contains_key(path2)
                    &&& path1.to_vaddr(self.constants.arch) == vbase
                    &&& path2.to_vaddr(self.constants.arch) == vbase
                } ==> path1 == path2,
    {
        assert forall|path1: PTTreePath, path2: PTTreePath|
            #![auto]
            {
                &&& self.path_mappings().contains_key(path1)
                &&& self.path_mappings().contains_key(path2)
                &&& path1.to_vaddr(self.constants.arch) == vbase
                &&& path2.to_vaddr(self.constants.arch) == vbase
            } implies path1 == path2 by {
            if path1 != path2 {
                // Proof by contradiction
                self.lemma_path_mappings_nonprefix();
                PTTreePath::lemma_nonprefix_implies_vaddr_inequality(
                    self.constants.arch,
                    path1,
                    path2,
                );
                assert(path1.to_vaddr(self.constants.arch) != path2.to_vaddr(self.constants.arch));
            }
        }
    }

    /// Lemma. `path_mappings` after `recursive_insert` contains the new mapping.
    pub proof fn lemma_path_mappings_after_insertion_contains_new_mapping(
        self,
        path: PTTreePath,
        frame: Frame,
    )
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            Self::is_entry_valid(
                NodeEntry::Frame(frame),
                (self.level + path.len() - 1) as nat,
                self.constants,
            ),
            self.recursive_insert(path, frame).1 is Ok,
        ensures
            self.recursive_insert(path, frame).0.path_mappings().contains_pair(path, frame),
        decreases path.len(),
    {
        let new = self.recursive_insert(path, frame);
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        assert(self.entries.contains(entry));
        if path.len() <= 1 {
            match entry {
                NodeEntry::Empty => assert(new.0.recursive_visit(path) == seq![
                    NodeEntry::Frame(frame),
                ]),
                _ => (),  // unreachable
            }
        } else {
            match entry {
                NodeEntry::Node(node) => {
                    assert(Self::is_entry_valid(entry, self.level, self.constants));
                    assert(node.invariants());
                    // Recursively prove `node.recursive_insert(remain, frame)`
                    node.lemma_path_mappings_after_insertion_contains_new_mapping(remain, frame);
                },
                NodeEntry::Empty => {
                    let node = PTTreeNode::new(self.constants, self.level + 1);
                    // `node` satisfies invariants by construction
                    assert(node.invariants());
                    // Recursively prove `node.recursive_insert(remain, frame)`
                    node.lemma_path_mappings_after_insertion_contains_new_mapping(remain, frame);
                },
                _ => (),  // unreachable
            }
        }
    }

    /// Lemma. `path_mappings` after `recursive_insert` is a superset of `path_mappings` before.
    pub proof fn lemma_path_mappings_after_insertion_is_superset(
        self,
        path: PTTreePath,
        frame: Frame,
    )
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            Self::is_entry_valid(
                NodeEntry::Frame(frame),
                (self.level + path.len() - 1) as nat,
                self.constants,
            ),
        ensures
            forall|path2: PTTreePath, frame2: Frame| #[trigger]
                self.path_mappings().contains_pair(path2, frame2) ==> self.recursive_insert(
                    path,
                    frame,
                ).0.path_mappings().contains_pair(path2, frame2),
        decreases path.len(),
    {
        let new = self.recursive_insert(path, frame).0;
        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            self.path_mappings().contains_pair(
                path2,
                frame2,
            ) implies new.path_mappings().contains_pair(path2, frame2) by {
            let (idx, remain) = path.step();
            let (idx2, remain2) = path2.step();
            let entry = self.entries[idx as int];
            assert(self.entries.contains(entry));
            // Postcondition satisfied obviously if `path.len() == 1`
            if path.len() > 1 {
                if idx == idx2 {
                    // `path` and `path2` share same step
                    match entry {
                        NodeEntry::Node(node) => {
                            assert(node.path_mappings().contains_pair(remain2, frame2));
                            // Recursively prove `node.recursive_insert(remain, frame)`
                            node.lemma_path_mappings_after_insertion_is_superset(remain, frame);
                            assert(node.recursive_insert(
                                remain,
                                frame,
                            ).0.path_mappings().contains_pair(remain2, frame2));
                        },
                        _ => assert(new == self),
                    }
                } else {
                    // `path` and `path2` do not share any prefix
                    match entry {
                        NodeEntry::Node(node) => assert(new == self.update(
                            idx,
                            NodeEntry::Node(node.recursive_insert(remain, frame).0),
                        )),
                        NodeEntry::Empty => assert(new == self.update(
                            idx,
                            NodeEntry::Node(
                                Self::new(self.constants, self.level + 1).recursive_insert(
                                    remain,
                                    frame,
                                ).0,
                            ),
                        )),
                        _ => assert(new == self),
                    }
                    // `self.entries` only updates at `idx`
                    assert(self.entries[idx2 as int] == new.entries[idx2 as int]);
                }
            }
        }
    }

    // Lemma. `recursive_insert` does not affect other mappings.
    pub proof fn lemma_insert_not_affect_other_mappings(self, path: PTTreePath, frame: Frame)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            Self::is_entry_valid(
                NodeEntry::Frame(frame),
                (self.level + path.len() - 1) as nat,
                self.constants,
            ),
            self.recursive_insert(path, frame).1 is Ok,
        ensures
            forall|path2: PTTreePath, frame2: Frame| #[trigger]
                self.recursive_insert(path, frame).0.path_mappings().contains_pair(path2, frame2)
                    ==> path2 == path || self.path_mappings().contains_pair(path2, frame2),
        decreases path.len(),
    {
        let new = self.recursive_insert(path, frame).0;
        self.lemma_insert_preserves_invariants(path, frame);

        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            new.path_mappings().contains_pair(path2, frame2) implies path2 == path
            || self.path_mappings().contains_pair(path2, frame2) by {
            let (idx, remain) = path.step();
            let (idx2, remain2) = path2.step();

            let entry = self.entries[idx as int];
            assert(self.entries.contains(entry));
            let new_entry = new.entries[idx as int];
            assert(new.entries.contains(new_entry));
            let entry2 = self.entries[idx2 as int];
            assert(self.entries.contains(entry2));
            let new_entry2 = new.entries[idx2 as int];
            assert(new.entries.contains(new_entry2));

            if path.len() <= 1 {
                // Base case, `new_entry` is the inserted frame while other entries unchanged
                assert(new_entry == NodeEntry::Frame(frame));
                if idx == idx2 {
                    assert(path2.0 == path.0);
                } else {
                    // `path2` points to an unchanged entry
                    assert(self.path_mappings().contains_pair(path2, frame2));
                }
            } else {
                if idx == idx2 {
                    // `path` and `path2` share same step
                    match (entry, new_entry) {
                        (NodeEntry::Node(node), NodeEntry::Node(new_node)) => {
                            // `node` becomes `new_node` after `recursive_insert`
                            assert(new_node == node.recursive_insert(remain, frame).0);
                            assert(new_node.path_mappings().contains_pair(remain2, frame2));
                            // Recursive prove that `new_node` has one more mapping than `node`
                            node.lemma_insert_not_affect_other_mappings(remain, frame);
                            assert(remain == remain2 || node.path_mappings().contains_pair(
                                remain2,
                                frame2,
                            ));
                            if remain == remain2 {
                                path.lemma_eq_step(path2);
                            } else {
                                assert(self.path_mappings().contains_pair(path2, frame2));
                            }
                        },
                        (NodeEntry::Empty, NodeEntry::Node(new_node)) => {
                            let node = Self::new(self.constants, self.level + 1);
                            // `node` becomes `new_node` after `recursive_insert`
                            assert(new_node == node.recursive_insert(remain, frame).0);
                            assert(new_node.path_mappings().contains_pair(remain2, frame2));
                            // Recursive prove that `new_node` has one more mapping than `node`
                            node.lemma_insert_not_affect_other_mappings(remain, frame);
                            assert(remain == remain2 || node.path_mappings().contains_pair(
                                remain2,
                                frame2,
                            ));
                            if remain == remain2 {
                                path.lemma_eq_step(path2);
                            } else {
                                assert(self.path_mappings().contains_pair(path2, frame2));
                            }
                        },
                        (_, NodeEntry::Frame(_)) | (_, NodeEntry::Empty) => {
                            // `new.path_mappings()` must contain `(path, frame)`, so `new_entry`
                            // cannot be frame or empty at this level
                            self.lemma_path_mappings_after_insertion_contains_new_mapping(
                                path,
                                frame,
                            );
                            assert(false);  // unreachable
                        },
                        _ => assert(false),  // unreachable
                    }
                } else {
                    // `path` and `path2` do not share any prefix
                    match entry {
                        NodeEntry::Node(node) => assert(new == self.update(
                            idx,
                            NodeEntry::Node(node.recursive_insert(remain, frame).0),
                        )),
                        NodeEntry::Empty => assert(new == self.update(
                            idx,
                            NodeEntry::Node(
                                Self::new(self.constants, self.level + 1).recursive_insert(
                                    remain,
                                    frame,
                                ).0,
                            ),
                        )),
                        _ => (),  // unreachable
                    }
                    // `self.entries` only updates at `idx`
                    assert(entry2 == new_entry2);
                    assert(self.path_mappings().contains_pair(path2, frame2));
                }
            }
        }
    }

    /// Lemma. `recursive_insert` adds a mapping to `path_mappings`.
    pub proof fn lemma_insert_adds_path_mapping(self, path: PTTreePath, frame: Frame)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            Self::is_entry_valid(
                NodeEntry::Frame(frame),
                (self.level + path.len() - 1) as nat,
                self.constants,
            ),
            self.recursive_insert(path, frame).1 is Ok,
        ensures
            self.recursive_insert(path, frame).0.path_mappings() == self.path_mappings().insert(
                path,
                frame,
            ),
    {
        let new = self.recursive_insert(path, frame).0;
        // `new.path_mappings()` contains the new mapping `(path, frame)`
        self.lemma_path_mappings_after_insertion_contains_new_mapping(path, frame);
        // `new.path_mappings()` is a superset of `self.path_mappings()`
        self.lemma_path_mappings_after_insertion_is_superset(path, frame);
        // Other mappings are preserved
        self.lemma_insert_not_affect_other_mappings(path, frame);

        // `new.path_mappings()` is a subset of `self.path_mappings().insert(path, frame)`
        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            new.path_mappings().contains_pair(path2, frame2) implies self.path_mappings().insert(
            path,
            frame,
        ).contains_pair(path2, frame2) by {
            if path2 == path {
                assert(frame2 == frame);
            }
        }
        // `self.path_mappings().insert(path, frame)` is a subset of `new.path_mappings()`
        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            self.path_mappings().insert(path, frame).contains_pair(
                path2,
                frame2,
            ) implies new.path_mappings().contains_pair(path2, frame2) by {
            if path2 == path {
                assert(new.path_mappings().contains_pair(path, frame));
            } else {
                assert(self.path_mappings().contains_pair(path2, frame2));
            }
        }
        lemma_map_eq_pair(self.path_mappings().insert(path, frame), new.path_mappings());
    }

    /// Lemma. `path_mappings` after removal does not contain the removed mapping.
    pub proof fn lemma_path_mappings_after_removal_not_contain_mapping(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            !self.recursive_remove(path).0.path_mappings().contains_key(path),
        decreases path.len(),
    {
        let new = self.recursive_remove(path).0;
        let (idx, remain) = path.step();
        let entry = self.entries[idx as int];
        assert(self.entries.contains(entry));
        if path.len() <= 1 {
            match entry {
                NodeEntry::Frame(_) => assert(new.recursive_visit(path) == seq![NodeEntry::Empty]),
                _ => assert(!new.is_frame_path(path)),
            }
        } else {
            match entry {
                NodeEntry::Node(node) => {
                    assert(Self::is_entry_valid(entry, self.level, self.constants));
                    assert(node.invariants());
                    // Recursively prove `node.recursive_remove(remain)`
                    node.lemma_path_mappings_after_removal_not_contain_mapping(remain);
                },
                _ => assert(!new.is_frame_path(path)),
            }
        }
    }

    /// Lemma. `path_mappings` after `recursive_remove` is subset of before.
    pub proof fn lemma_path_mappings_after_removal_is_subset(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            forall|path2: PTTreePath, frame2: Frame| #[trigger]
                self.recursive_remove(path).0.path_mappings().contains_pair(path2, frame2)
                    ==> self.path_mappings().contains_pair(path2, frame2),
        decreases path.len(),
    {
        let new = self.recursive_remove(path).0;
        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            new.path_mappings().contains_pair(
                path2,
                frame2,
            ) implies self.path_mappings().contains_pair(path2, frame2) by {
            let (idx, remain) = path.step();
            let (idx2, remain2) = path2.step();
            let entry = self.entries[idx as int];
            assert(self.entries.contains(entry));
            // Postcondition satisfied obviously if `path.len() == 1`
            if path.len() > 1 {
                if idx == idx2 {
                    // `path` and `path2` share same step
                    match entry {
                        NodeEntry::Node(node) => {
                            assert(node.recursive_remove(remain).0.path_mappings().contains_pair(
                                remain2,
                                frame2,
                            ));
                            // Recursively prove `node.recursive_remove(remain)`
                            node.lemma_path_mappings_after_removal_is_subset(remain);
                            assert(node.path_mappings().contains_pair(remain2, frame2));
                        },
                        _ => (),
                    }
                } else {
                    // `path` and `path2` do not share any prefix
                    match entry {
                        NodeEntry::Node(node) => assert(new == self.update(
                            idx,
                            NodeEntry::Node(node.recursive_remove(remain).0),
                        )),
                        _ => (),
                    }
                    // `self.entries` only updates at `idx`
                    assert(self.entries[idx2 as int] == new.entries[idx2 as int]);
                }
            }
        }
    }

    /// Lemma. `recursive_remove` does not affect other mappings.
    pub proof fn lemma_remove_not_affect_other_mappings(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            forall|path2: PTTreePath, frame2: Frame| #[trigger]
                self.path_mappings().contains_pair(path2, frame2) ==> path.has_prefix(path2)
                    || self.recursive_remove(path).0.path_mappings().contains_pair(path2, frame2),
        decreases path.len(),
    {
        let new = self.recursive_remove(path).0;
        self.lemma_remove_preserves_invariants(path);

        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            self.path_mappings().contains_pair(path2, frame2) implies path.has_prefix(path2)
            || new.path_mappings().contains_pair(path2, frame2) by {
            let (idx, remain) = path.step();
            let (idx2, remain2) = path2.step();

            let entry = self.entries[idx as int];
            assert(self.entries.contains(entry));
            let new_entry = new.entries[idx as int];
            assert(new.entries.contains(new_entry));
            let entry2 = self.entries[idx2 as int];
            assert(self.entries.contains(entry2));
            let new_entry2 = new.entries[idx2 as int];
            assert(new.entries.contains(new_entry2));

            if path.len() <= 1 {
                if idx == idx2 {
                    match entry {
                        NodeEntry::Frame(_) => {
                            assert(new.recursive_visit(path2) == seq![NodeEntry::Empty]);
                            // assert(path2.len() == 1);
                            assert(path.0 == path2.0);
                        },
                        _ => assert(new == self),
                    }
                } else {
                    // `path2` points to an unchanged entry
                    assert(new.path_mappings().contains_pair(path2, frame2));
                }
            } else {
                if idx == idx2 {
                    // `path` and `path2` share same step
                    match (entry, new_entry) {
                        (NodeEntry::Node(node), NodeEntry::Node(new_node)) => {
                            // `node` becomes `new_node` after `recursive_remove`
                            assert(new_node == node.recursive_remove(remain).0);
                            assert(node.path_mappings().contains_pair(remain2, frame2));
                            // Recursive prove that `new_node` has one less mapping than `node`
                            node.lemma_remove_not_affect_other_mappings(remain);
                            assert(remain.has_prefix(remain2)
                                || new_node.path_mappings().contains_pair(remain2, frame2));
                            if remain.has_prefix(remain2) {
                                path.lemma_prefix_step(path2);
                                assert(path.has_prefix(path2));
                            } else {
                                assert(new.path_mappings().contains_pair(path2, frame2));
                            }
                        },
                        (NodeEntry::Node(node), NodeEntry::Frame(_))
                        | (NodeEntry::Node(node), NodeEntry::Empty) => {
                            assert(new_entry == NodeEntry::Node(node.recursive_remove(remain).0));
                            assert(false);  // unreachable
                        },
                        (NodeEntry::Frame(_), _) => {
                            if remain.0 == seq![0nat; remain.len()] {
                                assert(new.recursive_visit(path2) == seq![NodeEntry::Empty]);
                                assert(self.recursive_visit(path2) == seq![entry]);
                            }
                        },
                        _ => (),
                    }
                } else {
                    // `path` and `path2` do not share any prefix
                    match entry {
                        NodeEntry::Node(node) => assert(new == self.update(
                            idx,
                            NodeEntry::Node(node.recursive_remove(remain).0),
                        )),
                        _ => (),
                    }
                    // `self.entries` only updates at `idx`
                    assert(entry2 == new_entry2);
                    assert(new.path_mappings().contains_pair(path2, frame2));
                }
            }
        }
    }

    /// Lemma. `recursive_remove` removes a mapping from `path_mappings`.
    pub proof fn lemma_remove_removes_path_mapping(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
        ensures
            self.recursive_remove(path).0.path_mappings() == self.path_mappings().remove(path),
    {
        let new = self.recursive_remove(path).0;
        self.lemma_remove_preserves_invariants(path);

        // `new.path_mappings()` does not contain the mapping `(path, frame)`
        self.lemma_path_mappings_after_removal_not_contain_mapping(path);
        // `new.path_mappings()` is a subset of `self.path_mappings()`
        self.lemma_path_mappings_after_removal_is_subset(path);
        // Other mappings are preserved
        self.lemma_remove_not_affect_other_mappings(path);

        // `new.path_mappings()` is a subset of `self.path_mappings().remove(path)`
        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            new.path_mappings().contains_pair(path2, frame2) implies self.path_mappings().remove(
            path,
        ).contains_pair(path2, frame2) by {
            assert(path2 != path);
        }
        // `self.path_mappings().remove(path)` is a subset of `new.path_mappings()`
        assert forall|path2: PTTreePath, frame2: Frame| #[trigger]
            self.path_mappings().remove(path).contains_pair(
                path2,
                frame2,
            ) implies new.path_mappings().contains_pair(path2, frame2) by {
            assert(path2 != path);
            assert(self.path_mappings().contains_pair(path2, frame2));
        }
        lemma_map_eq_pair(self.path_mappings().remove(path), new.path_mappings());
    }

    /// Lemma. `recursive_insert` fails for `path` implies `self.path_mappings()`
    /// contains `path2` such that `path2` is a prefix of `path` or `path` is a
    /// prefix of `path2`.
    pub proof fn lemma_insert_fails_implies_prefix(self, path: PTTreePath, frame: Frame)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            Self::is_entry_valid(
                NodeEntry::Frame(frame),
                (self.level + path.len() - 1) as nat,
                self.constants,
            ),
            self.recursive_insert(path, frame).1 is Err,
        ensures
            exists|path2: PTTreePath| #[trigger]
                self.path_mappings().contains_key(path2) && (path2.has_prefix(path)
                    || path.has_prefix(path2)),
        decreases path.len(),
    {
        // TODO
        assume(false);
    }

    /// Lemma. If `path` is contained in `self.path_mappings()`, and `path` is a real prefix of `path2`,
    /// then `self.recursive_remove(path2)` succeeds.
    pub proof fn lemma_remove_real_prefix_ok(self, path: PTTreePath, path2: PTTreePath)
        requires
            self.invariants(),
            self.path_mappings().contains_key(path),
            path2.valid(self.constants.arch, self.level),
            path2.has_real_prefix(path),
        ensures
            self.recursive_remove(path2).1 is Ok,
    {
        // TODO
        assume(false);
    }

    /// Lemma. If `self.recursive_remove(path)` succeeds, then `self.recursive_visit(path)` reaches a frame.
    pub proof fn lemma_remove_ok_implies_visit_reaches_frame(self, path: PTTreePath)
        requires
            self.invariants(),
            path.valid(self.constants.arch, self.level),
            self.recursive_remove(path).1 is Ok,
        ensures
            self.recursive_visit(path).last() is Frame,
            path.has_zero_tail(self.recursive_visit(path).len()),
    {
        // TODO
        assume(false);
    }
}

} // verus!
