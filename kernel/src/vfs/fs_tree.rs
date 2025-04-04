use core::sync::atomic::AtomicU64;
use std::{boxed::Box, collections::btree_map::BTreeMap, sync::mutex::Mutex, vec::Vec};

use super::{Inode, ResolvedPath, VFS};

pub(super) static INODE_CACHE: Mutex<InodeCache> = Mutex::new(InodeCache::new());
pub(super) static CURRENT_NUM: AtomicU64 = AtomicU64::new(0);

struct FsTreeNode {
    children: Vec<(Box<str>, InodeCacheNum)>,
}

pub(super) struct InodeCache {
    inodes: BTreeMap<InodeCacheNum, (Inode, FsTreeNode)>,
    root: InodeCacheNum,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InodeCacheNum(u64);

impl InodeCache {
    pub const fn new() -> Self {
        InodeCache {
            inodes: BTreeMap::new(),
            root: InodeCacheNum(0),
        }
    }
}

///Should be called when mounting a new fs as root
pub fn init(root: Inode) {
    let mut cache = INODE_CACHE.lock();
    let cache_num = InodeCacheNum(CURRENT_NUM.fetch_add(1, core::sync::atomic::Ordering::Relaxed));
    cache.inodes.clear();
    cache.inodes.insert(cache_num, (root, FsTreeNode { children: Vec::new() }));
    cache.root = cache_num;
}

pub fn get_inode(inode_num: InodeCacheNum) -> Option<Inode> {
    let cache = &mut *INODE_CACHE.lock();
    cache.inodes.get(&inode_num).map(|(inode, _)| inode).cloned()
}

pub fn get_inode_num(path: ResolvedPath) -> Option<InodeCacheNum> {
    let cache = &mut *INODE_CACHE.lock();
    let mut current = cache.root;
    for component in path.0.iter() {
        let no_children = cache.inodes.get(&current).unwrap().1.children.is_empty();
        if no_children {
            load_dir(current, &mut cache.inodes);
        }
        let current_node_inner = &cache.inodes.get(&current).unwrap().1;
        let mut found = false;
        for (name, node) in current_node_inner.children.iter() {
            if name == component {
                current = *node;
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    Some(current)
}

fn load_dir(current: InodeCacheNum, cache_inodes: &mut BTreeMap<InodeCacheNum, (Inode, FsTreeNode)>) {
    let inode = cache_inodes.get(&current).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.0.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    let dir = fs.read_dir(&inode.0);
    let mut children = Vec::new();
    if dir.is_empty() {
        return;
    }
    for dir_entry in dir.iter() {
        let inode = fs.stat(dir_entry.inode);
        let cache_num = InodeCacheNum(CURRENT_NUM.fetch_add(1, core::sync::atomic::Ordering::Relaxed));
        cache_inodes.insert(cache_num, (inode, FsTreeNode { children: Vec::new() }));
        children.push((dir_entry.name.clone(), cache_num));
    }
    cache_inodes.get_mut(&current).unwrap().1.children = children;
}

pub fn update_inode(cache_num: InodeCacheNum, inode: Inode) {
    let mut cache = INODE_CACHE.lock();
    cache.inodes.get_mut(&cache_num).unwrap().0 = inode;
}

pub fn insert_inode(parent_cache_num: InodeCacheNum, name: Box<str>, inode: Inode) -> InodeCacheNum {
    let mut cache = INODE_CACHE.lock();
    let cache_num = InodeCacheNum(CURRENT_NUM.fetch_add(1, core::sync::atomic::Ordering::Relaxed));
    cache.inodes.insert(cache_num, (inode, FsTreeNode { children: Vec::new() }));
    let parent = cache.inodes.get_mut(&parent_cache_num).unwrap();
    parent.1.children.push((name, cache_num));
    cache_num
}
