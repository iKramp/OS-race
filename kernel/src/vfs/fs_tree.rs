use core::sync::atomic::AtomicU64;
use std::{boxed::Box, collections::btree_map::BTreeMap, sync::mutex::Mutex, vec::Vec};

use super::{Inode, ResolvedPath, VFS};

pub(super) static INODE_CACHE: Mutex<InodeCache> = Mutex::new(InodeCache::new());
pub(super) static CURRENT_NUM: AtomicU64 = AtomicU64::new(0);

struct FsTreeNode {
    cahce_num: u64,
    children: Vec<(Box<str>, FsTreeNode)>,
}

pub(super) struct InodeCache {
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

pub fn get_inode(inode_num: u64) -> Option<Inode> {
    let cache = &mut *INODE_CACHE.lock();
    cache.inodes.get(&inode_num).cloned()
}

pub fn get_inode_num(path: ResolvedPath) -> Option<u64> {
    let cache = &mut *INODE_CACHE.lock();
    let mut current = &mut cache.root;
    let cache_inodes = &mut cache.inodes;
    for component in path.0.iter() {
        if current.children.is_empty() {
            load_dir(current, cache_inodes);
        }
        let mut found = false;
        for (name, node) in current.children.iter_mut() {
            if name == component {
                current = unsafe { &mut *(node as *mut FsTreeNode) };
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    Some(current.cahce_num)
}

fn load_dir(current: &mut FsTreeNode, cache_inodes: &mut BTreeMap<u64, Inode>) {
    let inode = cache_inodes.get(&current.cahce_num).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.device).unwrap();
    let partition_id = device_details.partition.clone();
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    let dir = fs.read_dir(inode);
    if dir.is_empty() {
        return;
    }
    for dir_entry in dir.iter() {
        let inode = fs.stat(dir_entry.inode);
        let cache_num = CURRENT_NUM.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        cache_inodes.insert(cache_num, inode);
        current.children.push((
            dir_entry.name.clone(),
            FsTreeNode {
                cahce_num: cache_num,
                children: Vec::new(),
            },
        ));
    }
}
