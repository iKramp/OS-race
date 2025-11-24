//!A module for a directory temporary filesystem (dtmpfs).
//!It is the root filesystem before any actual disk is mounted. It cannot store any files, but
//!provides a directory structure for mounpoints

use std::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    lock_w_info,
    string::{String, ToString},
    sync::arc::Arc,
    sync::no_int_spinlock::NoIntSpinlock,
    vec::Vec,
};

use uuid::Uuid;

use super::{
    DeviceId, InodeIndex, InodeType,
    filesystem_trait::{FileSystem, FileSystemFactory},
};

#[derive(Debug)]
pub(super) struct Dtmpfs {
    global_lock: NoIntSpinlock<()>,
    root: u64,
    inodes: BTreeMap<u64, DtmpfsNode>,
    inode_index: u64,
}

#[derive(Debug)]
struct DtmpfsNode {
    children: Vec<(String, u64)>, // (name, inode)
}

pub(super) struct DtmpfsFactory;

impl DtmpfsFactory {
    pub const UUID: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000000");
}

#[async_trait::async_trait]
impl FileSystemFactory for DtmpfsFactory {
    async fn mount(&self, _partition: crate::drivers::disk::MountedPartition) -> Arc<dyn FileSystem + Send> {
        let mut fs = Dtmpfs {
            global_lock: NoIntSpinlock::new(()),
            root: 2, // Root inode index
            inodes: BTreeMap::new(),
            inode_index: 3,
        };
        fs.inodes.insert(fs.root, DtmpfsNode { children: Vec::new() });
        Arc::new(fs)
    }
}

#[async_trait::async_trait]
impl FileSystem for Dtmpfs {
    async fn unmount(&self) {}

    async fn read(&self, _inode: InodeIndex, _offset_bytes: u64, _size_bytes: u64, _buffer: &[std::mem_utils::PhysAddr]) -> u64 {
        panic!("Reading is not supported in dtmpfs");
    }

    async fn read_dir(&self, inode: InodeIndex) -> std::boxed::Box<[crate::drivers::disk::DirEntry]> {
        let lock = lock_w_info!(self.global_lock);
        let mut entries = Vec::new();
        if let Some(node) = self.inodes.get(&inode) {
            for (name, child_inode) in &node.children {
                entries.push(crate::drivers::disk::DirEntry {
                    name: name.clone().into_boxed_str(),
                    inode: *child_inode,
                });
            }
        }
        drop(lock);
        entries.into_boxed_slice()
    }

    async fn write(&self, _inode: InodeIndex, _offset: u64, _size: u64, _buffer: &[std::mem_utils::PhysAddr]) -> super::Inode {
        panic!("Writing is not supported in dtmpfs");
    }

    async fn stat(&self, inode: InodeIndex) -> super::Inode {
        unsafe {
            super::Inode {
                index: inode,
                device: DeviceId(0),
                type_mode: InodeType::new_dir(0o755), //rwxr-xr-x
                link_cnt: 0,
                uid: 0,
                gid: 0,
                size: 0,
                preferred_block_size: 0,
                blocks: 0,
                access_time: 0,
                modification_time: 0,
                stat_change_time: 0,
            }
        }
    }

    async fn set_stat(&self, _inode_index: InodeIndex, _inode_data: super::Inode) {
        panic!("Setting stat is not supported in dtmpfs");
    }

    async fn create(
        &self,
        name: &str,
        parent_dir: InodeIndex,
        _type_mode: super::InodeType,
        _uid: u16,
        _gid: u16,
    ) -> (super::Inode, super::Inode) {
        let lock = lock_w_info!(self.global_lock);
        #[allow(invalid_reference_casting)]
        let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        let inode_index = self.inode_index;
        self_mut.inode_index += 1;
        self_mut.inodes.insert(inode_index, DtmpfsNode { children: Vec::new() });

        let Some(parent_inode) = self_mut.inodes.get_mut(&parent_dir) else {
            panic!("Parent directory inode {} not found", parent_dir);
        };

        parent_inode.children.push((name.to_string(), inode_index));
        drop(lock);
        (self.stat(parent_dir).await, self.stat(inode_index).await)
    }

    async fn unlink(&self, parent_inode: InodeIndex, name: &str) {
        let lock = lock_w_info!(self.global_lock);
        #[allow(invalid_reference_casting)]
        let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        if let Some(parent_node) = self_mut.inodes.get_mut(&parent_inode) {
            parent_node.children.retain(|(n, _)| n != name);
        }
        drop(lock);
    }

    async fn link(&self, _inode: InodeIndex, _parent_dir: InodeIndex, _name: &str) -> super::Inode {
        panic!("Linking is not supported in dtmpfs");
    }

    async fn truncate(&self, _inode: InodeIndex, _size: u64) {
        panic!("Truncating is not supported in dtmpfs");
    }

    async fn rename(&self, inode: InodeIndex, parent_inode: InodeIndex, name: &str) {
        let lock = lock_w_info!(self.global_lock);
        #[allow(invalid_reference_casting)]
        let self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        let Some(parent_node) = self_mut.inodes.get_mut(&parent_inode) else {
            return;
        };
        if let Some((_, child_inode)) = parent_node.children.iter_mut().find(|(n, _)| n == name) {
            *child_inode = inode;
        } else {
            parent_node.children.push((name.to_string(), inode));
        }
        drop(lock);
    }
}
