use core::sync::atomic::AtomicU64;
use std::{boxed::Box, collections::btree_map::BTreeMap, sync::mutex::Mutex, vec::Vec};

use super::Inode;

static INODE_CACHE: Mutex<InodeCache> = Mutex::new(InodeCache::new());
pub(super) static CURRENT_NUM: AtomicU64 = AtomicU64::new(0);

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
    let mut cache = INODE_CACHE.lock();
    let cache_num = CURRENT_NUM.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    cache.inodes.clear();
    cache.inodes.insert(cache_num, root);
    cache.root = FsTreeNode {
        cahce_num: cache_num,
        children: Vec::new(),
    };
}
