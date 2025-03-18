//! Tree model of the page table.
use vstd::prelude::*;

use crate::spec::{
    addr::VAddr,
    frame::{Frame, FrameSize},
};

verus! {

/// Page table architecture level.
pub struct PTArchLevel {
    /// The number of entries at this level.
    pub num_entries: nat,
    /// Frame size indicated by a block/page descriptor at this level.
    pub frame_size: FrameSize,
}

/// Page table architecture.
pub struct PTArch(pub Seq<PTArchLevel>);

impl PTArch {
    /// The number of levels in the page table.
    pub open spec fn levels(&self) -> nat {
        self.0.len()
    }

    /// The number of entries at a given level.
    pub open spec fn num_entries(&self, level: nat) -> nat {
        self.0[level as int].num_entries
    }

    /// The frame size indicated by a block/page descriptor at a given level.
    pub open spec fn frame_size(&self, level: nat) -> FrameSize {
        self.0[level as int].frame_size
    }
}

/// For VMSAv8-64 using 4K granule. The architecture is specified as follows:
///
/// | Level | Index into PT | Entry Num |  Entry Type  | Frame Size |
/// |-------|---------------|-----------|--------------|------------|
/// | 0     | 47:39         | 512       | Table/Block* | 512G       |
/// | 1     | 38:30         | 512       | Table/Block  | 1G         |
/// | 2     | 29:21         | 512       | Table/Block  | 2M         |
/// | 3     | 20:12         | 512       | Page         | 4K         |
/// 
/// *If effective value of TCR_ELx.DS is 0, level 0 allows Table descriptor only.
pub spec const VMSAV8_ARCH: PTArch = PTArch(
    seq![
        PTArchLevel { num_entries: 512, frame_size: FrameSize::Size512G },
        PTArchLevel { num_entries: 512, frame_size: FrameSize::Size1G },
        PTArchLevel { num_entries: 512, frame_size: FrameSize::Size2M },
        PTArchLevel { num_entries: 512, frame_size: FrameSize::Size4K },
    ],
);

/// Represents a node in the page table tree, which can be either an intermediate node
/// or a leaf node mapping to a physical frame.
pub tracked struct PTTreeNode {
    /// The entries of the node, which can be either sub-nodes, frames, or empty entries.
    pub entries: Seq<NodeEntry>,
    /// The base virtual address of the node, indicating the starting address of the virtual
    /// address range managed by this node.
    pub base: VAddr,
    /// The level of the node in the page table hierarchy.
    pub level: nat,
    /// The architecture of the page table.
    pub arch: PTArch,
}

/// Represents an entry in the page table node, which can be a sub-node, a physical frame,
/// or an empty entry.
pub tracked enum NodeEntry {
    /// A sub-node in the page table, representing an intermediate level of the page table hierarchy.
    Node(PTTreeNode),
    /// A physical frame mapped by the node, representing a leaf node in the page table tree.
    Frame(Frame),
    /// An empty entry in the page table, indicating that the corresponding virtual address range
    /// is not currently mapped or allocated.
    Empty,
}

impl PTTreeNode {
    /// Invariants. Recursively checks the invariants of the node and its sub-nodes.
    pub open spec fn invariants(self) -> bool 
        decreases self.arch.levels() - self.level
    {
        &&& self.level < self.arch.levels()
        &&& self.entries.len() == self.arch.num_entries(self.level)
        &&& if self.level == self.arch.levels() - 1 {
            // Leaf node
            forall |entry: NodeEntry| self.entries.contains(entry) ==> match entry {
                NodeEntry::Node(_) => false, // Leaf node cannot have sub-nodes
                NodeEntry::Frame(frame) => {
                    &&& frame.size == self.arch.frame_size(self.level)
                    &&& frame.base.aligned(frame.size.as_nat())
                }
                NodeEntry::Empty => true,
            }
        } else {
            // Intermediate node
            forall |entry: NodeEntry| self.entries.contains(entry) ==> match entry {
                NodeEntry::Node(node) => {
                    &&& node.level == self.level + 1
                    &&& node.arch == self.arch
                    &&& node.invariants()
                }
                NodeEntry::Frame(frame) => {
                    &&& frame.size == self.arch.frame_size(self.level)
                    &&& frame.base.aligned(frame.size.as_nat())
                }
                NodeEntry::Empty => true,
            }
        }
    }

    /// Creates an empty root node.
    pub open spec fn new_root(base: VAddr, arch: PTArch) -> Self {
        Self { entries: seq![NodeEntry::Empty; arch.num_entries(0)], base, level: 0, arch }
    }

    /// Creates an empty node.
    pub open spec fn new(base: VAddr, level: nat, arch: PTArch) -> Self {
        Self { entries: seq![NodeEntry::Empty; arch.num_entries(level)], base, level, arch }
    }

    /// Theorem. `new` function implies invariants.
    pub proof fn new_implies_invariants(base: VAddr, level: nat, arch: PTArch)
        requires
            level < arch.levels(),
        ensures
            Self::new(base, level, arch).invariants(),
    {
    }

    /// Insert an entry into the node at the specified index.
    pub open spec fn insert(self, index: int, entry: NodeEntry) -> Self {
        Self { entries: self.entries.update(index, entry), ..self }
    }

    /// Theorem. `insert` function preserves invariants.
    pub proof fn insert_preserves_invariants(self, index: int, entry: NodeEntry)
        requires
            self.invariants(),
        ensures
            self.insert(index, entry).invariants(),
    {
    }

    /// Remove an entry from the node at the specified index.
    pub open spec fn remove(self, index: int) -> Self {
        Self { entries: self.entries.remove(index), ..self }
    }

    /// Theorem. `remove` function preserves invariants.
    pub proof fn remove_preserves_invariants(self, index: int)
        requires
            self.invariants(),
        ensures
            self.remove(index).invariants(),
    {
    }
}

} // verus!
