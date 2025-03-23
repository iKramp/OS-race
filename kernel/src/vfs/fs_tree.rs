use std::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use super::Inode;

static mut INODE_CACHE: InodeCache = InodeCache::new();
pub(super) static mut CURRENT_NUM: u64 = 0;

struct FsTreeNode {
    cahce_num: u64,
    children: Vec<(Box<str>, FsTreeNode)>,
}

struct InodeCache {
    inodes: BTreeMap<u64, Inode>,
    root: FsTreeNode,
}

impl InodeCache {
    pub const fn new() -> Self {
        InodeCache {
            inodes: BTreeMap::new(),
            root: FsTreeNode {
                cahce_num: 0,
                children: Vec::new(),
            },
        }
    }
}

///Should be called when mounting a new fs as root
pub fn init(root: Inode) {
    unsafe {
        INODE_CACHE.inodes.clear();
        INODE_CACHE.inodes.insert(CURRENT_NUM, root);
        INODE_CACHE.root = FsTreeNode {
            cahce_num: CURRENT_NUM,
            children: Vec::new(),
        };
        CURRENT_NUM += 1;
    }
}
