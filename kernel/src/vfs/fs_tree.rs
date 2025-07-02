use std::{boxed::Box, collections::btree_map::BTreeMap, sync::mutex::Mutex, vec::Vec};

use super::{DeviceId, Inode, InodeIndex, ResolvedPathBorrowed, VFS};

pub(super) static INODE_CACHE: Mutex<InodeCache> = Mutex::new(InodeCache::new());

struct FsTreeNode {
    children: Vec<(Box<str>, InodeIndex)>,
}

pub(super) struct InodeCache {
    inodes: BTreeMap<InodeIndex, (Inode, FsTreeNode)>,
    root: InodeIndex,
    ///maps from parent inode in mount point to child inode in mount point
    mount_points: BTreeMap<InodeIndex, InodeIndex>,
}

impl InodeCache {
    pub const fn new() -> Self {
        InodeCache {
            inodes: BTreeMap::new(),
            root: InodeIndex {
                device_id: DeviceId(0),
                index: 0,
            },
            mount_points: BTreeMap::new(),
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

pub fn get_inode_index(path: ResolvedPathBorrowed) -> Option<InodeIndex> {
    let cache = &mut *INODE_CACHE.lock();
    let mut current = cache.root;
    for component in path.iter() {
        if let Some(mount_point) = cache.mount_points.get(&current) {
            current = *mount_point;
        }
        let current_node = cache.inodes.get(&current)?;
        let child = current_node.1.children.iter().find(|(name, _)| name == component);
        if let Some(child) = child {
            current = child.1;
            continue;
        }
        // If the child is not found, we need to load the directory
        load_dir(current, &mut cache.inodes);
        // After loading, we check again
        let current_node = cache.inodes.get(&current)?;
        let child = current_node.1.children.iter().find(|(name, _)| name == component);
        if let Some(child) = child {
            current = child.1;
            continue;
        }
        return None;
    }
    if let Some(mount_point) = cache.mount_points.get(&current) {
        current = *mount_point;
    }
    Some(current)
}

fn load_dir(current: InodeIndex, cache_inodes: &mut BTreeMap<InodeIndex, (Inode, FsTreeNode)>) {
    let inode = cache_inodes.get(&current).unwrap();
    let mut vfs = VFS.lock();
    let device_details = vfs.devices.get(&inode.0.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_partitions.get_mut(&partition_id).unwrap();
    let dir = fs.read_dir(current.index as u32);
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

///parent_cache_num refers to the mountpoint itself, on top of which the new inode will be mounted
pub fn mount_inode(parent_cache_num: InodeIndex, inode: Inode) {
    let mut cache = INODE_CACHE.lock();
    let inode_index = InodeIndex {
        device_id: inode.device,
        index: inode.index as u64,
    };
    cache.inodes.insert(inode_index, (inode, FsTreeNode { children: Vec::new() }));
    cache.mount_points.insert(parent_cache_num, inode_index);
}

///parent_cache_num refers to the parent directory, NOT the mountpoint itself
///returns true if the last mountpoint of this filesystem was unmounted
pub fn unmount_inode(parent_cache_num: InodeIndex, name: &str) -> bool {
    let mut cache = INODE_CACHE.lock();
    let Some(parent) = cache.inodes.get_mut(&parent_cache_num) else {
        return false;
    };
    let Some(child) = parent.1.children.iter().find(|(n, _)| n.as_ref() == name) else {
        return false;
    };
    let index = child.1;
    let unmounted_device = cache.mount_points.remove(&index).map_or(DeviceId(u64::MAX), |v| v.device_id);
    let count = cache
        .mount_points
        .values()
        .filter(|&&v| v.device_id == unmounted_device)
        .count();
    drop(cache);
    if count == 0 {
        remove_device(unmounted_device);
        return true;
    }
    false
}

/// Removes all inodes associated with a specific device ID. Called when device is fully unmounted
pub fn remove_device(device_id: DeviceId) {
    let mut cache = INODE_CACHE.lock();
    cache.inodes.retain(|_, (inode, _)| inode.device != device_id);
    if cache.root.device_id == device_id {
        cache.root = InodeIndex {
            device_id: DeviceId(0),
            index: 0,
        };
    }
}
