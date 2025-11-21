use core::borrow::BorrowMut;
use std::{boxed::Box, collections::btree_map::BTreeMap, lock_w_info, printlnc, sync::{lock_info::LockLocationInfo, no_int_spinlock::{NoIntSpinlock, NoIntSpinlockGuard}}, vec::Vec};

use super::{DeviceId, Inode, InodeIdentifier, InodeIdentifierChain, ResolvedPathBorrowed, VFS};

pub(super) static INODE_CACHE: NoIntSpinlock<InodeCache> = NoIntSpinlock::new(InodeCache::new());

struct FsTreeNode {
    children: Vec<(Box<str>, InodeIdentifier)>,
}

pub(super) struct InodeCache {
    inodes: BTreeMap<InodeIdentifier, (Inode, FsTreeNode)>,
    root: InodeIdentifier,
    ///maps from parent inode in mount point to child inode in mount point
    mount_points: BTreeMap<InodeIdentifier, InodeIdentifier>,
}

impl InodeCache {
    pub const fn new() -> Self {
        InodeCache {
            inodes: BTreeMap::new(),
            root: InodeIdentifier {
                device_id: DeviceId::new(0),
                index: 0,
            },
            mount_points: BTreeMap::new(),
        }
    }
}

///Should be called when mounting a new fs as root
pub fn init(root: Inode) {
    let mut cache = lock_w_info!(INODE_CACHE);

    let inode_index = InodeIdentifier {
        device_id: root.device,
        index: root.index,
    };
    cache.inodes.clear();
    cache.inodes.insert(inode_index, (root, FsTreeNode { children: Vec::new() }));
    cache.root = inode_index;
}

pub fn get_inode(inode_index: InodeIdentifier) -> Option<Inode> {
    let cache = &mut lock_w_info!(INODE_CACHE);
    cache.inodes.get(&inode_index).map(|(inode, _)| inode).cloned()
}

pub async fn get_unmount_inodes(path: ResolvedPathBorrowed<'_>, from: Option<InodeIdentifier>) -> Option<(InodeIdentifier, InodeIdentifier)> {
    let mut cache = Some(lock_w_info!(INODE_CACHE));
    let mut current = from.unwrap_or(cache.as_ref().expect("is some").root);
    for component in path.iter() {
        while let Some(mount_point) = cache.as_ref().expect("is some").mount_points.get(&current) {
            if *mount_point == current {
                printlnc!((0, 0, 255), "Detected mount loop at inode {:?}\n", current);
                break;
            }
            current = *mount_point;
        }
        let child = find_child_no_mounts(current, component, &mut cache).await;
        if let Some(child) = child {
            current = child;
        } else {
            return None;
        }
    }
    let mut old = current;
    while let Some(mount_point) = cache.as_ref().expect("is some").mount_points.get(&current) {
        if *mount_point == current {
            printlnc!((0, 0, 255), "Detected mount loop at inode {:?}\n", current);
            break;
        }
        old = current;
        current = *mount_point;
    }
    if old == current {
        return None;
    }
    Some((old, current))
}

pub async fn get_inode_chain(path: ResolvedPathBorrowed<'_>, from: Option<InodeIdentifierChain>) -> Option<(InodeIdentifier, InodeIdentifierChain)> {
    let mut cache_lock = Some(lock_w_info!(INODE_CACHE));
    let mut current = from.unwrap_or(Box::new([cache_lock.as_ref().expect("is some").root])).to_vec();
    if current.is_empty() {
        panic!("Empty inode chain passed to get_inode_index");
    }
    for component in path.iter() {
        while let Some(mount_point) = cache_lock.as_ref().expect("is some").mount_points.get(current.last().unwrap()) {
            if mount_point == current.last().unwrap() {
                printlnc!((0, 0, 255), "Detected mount loop at inode {:?}\n", current);
                break;
            }
            *current.last_mut().unwrap() = *mount_point;
        }
        if **component == *".." {
            if current.len() > 1 {
                current.pop();
            }
            continue;
        }

        let child = find_child_no_mounts(*current.last().unwrap(), component, &mut cache_lock).await;
        if let Some(child) = child {
            current.push(child);
        } else {
            return None;
        }
    }
    if let Some(mount_point) = cache_lock.expect("is some").mount_points.get(current.last().unwrap()) {
        *current.last_mut().unwrap() = *mount_point;
    }
    let file = *current.last().unwrap();
    if current.len() > 1 {
        current.pop();
    }

    Some((file, current.into_boxed_slice()))
}

async fn find_child_no_mounts(current: InodeIdentifier, f_name: &str, cache: &mut Option<NoIntSpinlockGuard<'_, InodeCache>>) -> Option<InodeIdentifier> {
    let current_node = cache.as_ref().expect("is some").inodes.get(&current)?;
    let child = current_node.1.children.iter().find(|(name, _)| **name == *f_name);
    if let Some(child) = child {
        return Some(child.1);
    }
    // If the child is not found, we need to load the directory
    load_dir(current, cache).await;
    // After loading, we check again
    let current_node = cache.as_ref().expect("is some").inodes.get(&current)?;
    let child = current_node.1.children.iter().find(|(name, _)| **name == *f_name);
    if let Some(child) = child {
        return Some(child.1);
    }
    None
}

async fn load_dir(current: InodeIdentifier, cache: &mut Option<NoIntSpinlockGuard<'_, InodeCache>>) {
    let inode = cache.as_ref().expect("is some").inodes.get(&current).unwrap();
    let mut vfs = lock_w_info!(VFS);
    let device_details = vfs.devices.get(&inode.0.device).unwrap();
    let partition_id = device_details.partition;
    let fs = vfs.mounted_filesystems.get_mut(&partition_id).unwrap().clone();
    drop(vfs);
    drop(cache.take()); //drop lock

    let dir = fs.read_dir(current.index).await;

    let mut children = Vec::new();
    if dir.is_empty() {
        return;
    }
    for dir_entry in dir.iter() {
        drop(cache.take()); //drop in loop
        let inode = fs.stat(dir_entry.inode).await;
        let inode_index = InodeIdentifier {
            device_id: inode.device,
            index: inode.index,
        };
        *cache = Some(lock_w_info!(INODE_CACHE)); //get lock back
        cache.as_mut().expect("is some").inodes.insert(inode_index, (inode, FsTreeNode { children: Vec::new() }));
        children.push((dir_entry.name.clone(), inode_index));
    }
    cache.as_mut().expect("is some").inodes.get_mut(&current).unwrap().1.children = children;
}

pub fn update_inode(cache_num: InodeIdentifier, inode: Inode) {
    let mut cache = lock_w_info!(INODE_CACHE);
    cache.inodes.get_mut(&cache_num).unwrap().0 = inode;
}

pub fn insert_inode(parent_cache_num: InodeIdentifier, name: Box<str>, inode: Inode) {
    let mut cache = lock_w_info!(INODE_CACHE);
    let inode_index = InodeIdentifier {
        device_id: inode.device,
        index: inode.index,
    };
    cache.inodes.insert(inode_index, (inode, FsTreeNode { children: Vec::new() }));
    let parent = cache.inodes.get_mut(&parent_cache_num).unwrap();
    parent.1.children.push((name, inode_index));
}

///parent_cache_num refers to the mountpoint itself, on top of which the new inode will be mounted
pub fn mount_inode(parent_cache_num: InodeIdentifier, inode: Inode) {
    let mut cache = lock_w_info!(INODE_CACHE);
    let inode_index = InodeIdentifier {
        device_id: inode.device,
        index: inode.index,
    };
    cache.inodes.insert(inode_index, (inode, FsTreeNode { children: Vec::new() }));
    cache.mount_points.insert(parent_cache_num, inode_index);
}

///parent_cache_num refers to the parent directory, NOT the mountpoint itself
///returns true if the last mountpoint of this filesystem was unmounted
pub fn unmount_inode(parent_cache_num: InodeIdentifier) -> bool {
    let mut cache = lock_w_info!(INODE_CACHE);
    let unmounted_device = cache
        .mount_points
        .remove(&parent_cache_num)
        .map_or(DeviceId::new(u64::MAX), |v| v.device_id);
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
    let mut cache = lock_w_info!(INODE_CACHE);
    cache.inodes.retain(|_, (inode, _)| inode.device != device_id);
    if cache.root.device_id == device_id {
        cache.root = InodeIdentifier {
            device_id: DeviceId::new(0),
            index: 0,
        };
    }
}
