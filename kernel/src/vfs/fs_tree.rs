use std::{boxed::Box, collections::btree_map::BTreeMap, sync::mutex::Mutex, vec::Vec};

use super::{DeviceId, Inode, ResolvedPath, VFS};

pub(super) static INODE_CACHE: Mutex<InodeCache> = Mutex::new(InodeCache::new());

struct FsTreeNode {
    children: Vec<(Box<str>, InodeIndex)>,
}

pub(super) struct InodeCache {
    inodes: BTreeMap<InodeIndex, (Inode, FsTreeNode)>,
    root: InodeIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InodeIndex {
    pub device_id: DeviceId,
    pub index: u64,
}

impl InodeCache {
    pub const fn new() -> Self {
        InodeCache {
            inodes: BTreeMap::new(),
            root: InodeIndex {
                device_id: DeviceId(0),
                index: 0,
            },
        }
    }
}

///Should be called when mounting a new fs as root
pub fn init(root: Inode) {
    let mut cache = INODE_CACHE.lock();

    let inode_index = InodeIndex {
        device_id: root.device,
        index: root.index as u64,
    };
    cache.inodes.clear();
    cache.inodes.insert(inode_index, (root, FsTreeNode { children: Vec::new() }));
    cache.root = inode_index;
}

pub fn get_inode(inode_index: InodeIndex) -> Option<Inode> {
    let cache = &mut *INODE_CACHE.lock();
    cache.inodes.get(&inode_index).map(|(inode, _)| inode).cloned()
}

pub fn get_inode_index(path: ResolvedPath) -> Option<InodeIndex> {
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

fn load_dir(current: InodeIndex, cache_inodes: &mut BTreeMap<InodeIndex, (Inode, FsTreeNode)>) {
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
        let inode_index = InodeIndex {
            device_id: inode.device,
            index: inode.index as u64,
        };
        cache_inodes.insert(inode_index, (inode, FsTreeNode { children: Vec::new() }));
        children.push((dir_entry.name.clone(), inode_index));
    }
    cache_inodes.get_mut(&current).unwrap().1.children = children;
}

pub fn update_inode(cache_num: InodeIndex, inode: Inode) {
    let mut cache = INODE_CACHE.lock();
    cache.inodes.get_mut(&cache_num).unwrap().0 = inode;
}

pub fn insert_inode(parent_cache_num: InodeIndex, name: Box<str>, inode: Inode) {
    let mut cache = INODE_CACHE.lock();
    let inode_index = InodeIndex {
        device_id: inode.device,
        index: inode.index as u64,
    };
    cache.inodes.insert(inode_index, (inode, FsTreeNode { children: Vec::new() }));
    let parent = cache.inodes.get_mut(&parent_cache_num).unwrap();
    parent.1.children.push((name, inode_index));
}
