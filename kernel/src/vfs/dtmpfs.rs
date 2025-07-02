//!A module for a directory temporary filesystem (dtmpfs).
//!It is the root filesystem before any actual disk is mounted. It cannot store any files, but
//!provides a directory structure for mounpoints

use std::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use uuid::Uuid;

use crate::drivers::disk::{FileSystem, FileSystemFactory};

use super::InodeType;

pub(super) struct Dtmpfs {
    root: u32,
    inodes: BTreeMap<u32, DtmpfsNode>,
    inode_index: u32,
}

struct DtmpfsNode {
    children: Vec<(String, u32)>, // (name, inode)
}

pub(super) struct DtmpfsFactory;

impl DtmpfsFactory {
    pub const UUID: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000000");
}

impl FileSystemFactory for DtmpfsFactory {

    fn mount(&self, _partition: crate::drivers::disk::MountedPartition) -> std::boxed::Box<dyn FileSystem + Send> {
        let mut fs = Dtmpfs {
            root: 2, // Root inode index
            inodes: BTreeMap::new(),
            inode_index: 3,
        };
        fs.inodes.insert(fs.root, DtmpfsNode { children: Vec::new() });
        Box::new(fs)
    }
}

impl FileSystem for Dtmpfs {
    fn unmount(&mut self) {}

    fn read(&mut self, _inode: u32, _offset_bytes: u64, _size_bytes: u64, _buffer: &[std::mem_utils::PhysAddr]) {
        panic!("Reading is not supported in dtmpfs");
    }

    fn read_dir(&mut self, inode: u32) -> std::boxed::Box<[crate::drivers::disk::DirEntry]> {
        let mut entries = Vec::new();
        if let Some(node) = self.inodes.get(&inode) {
            for (name, child_inode) in &node.children {
                entries.push(crate::drivers::disk::DirEntry {
                    name: name.clone().into_boxed_str(),
                    inode: *child_inode,
                });
            }
        }
        entries.into_boxed_slice()
    }

    fn write(&mut self, _inode: u32, _offset: u64, _size: u64, _buffer: &[std::mem_utils::PhysAddr]) -> super::Inode {
        panic!("Writing is not supported in dtmpfs");
    }

    fn stat(&mut self, inode: u32) -> super::Inode {
        super::Inode {
            index: inode,
            device: super::DeviceId(0),
            type_mode: InodeType::new_dir(0o755), //rwxr-xr-x
            link_cnt: 0,
            uid: 0,
            gid: 0,
            device_represented: Some(super::DeviceId(0)),
            size: 0,
            preferred_block_size: 0,
            blocks: 0,
            access_time: 0,
            modification_time: 0,
            stat_change_time: 0,
        }
    }

    fn set_stat(&mut self, _inode_index: u32, _inode_data: super::Inode) {
        panic!("Setting stat is not supported in dtmpfs");
    }

    fn create(
        &mut self,
        name: &str,
        parent_dir: u32,
        _type_mode: super::InodeType,
        _uid: u16,
        _gid: u16,
    ) -> (super::Inode, super::Inode) {
        let inode_index = self.inode_index;
        self.inode_index += 1;
        self.inodes.insert(inode_index, DtmpfsNode { children: Vec::new() });

        let Some(parent_inode) = self.inodes.get_mut(&parent_dir) else {
            panic!("Parent directory inode {} not found", parent_dir);
        };

        parent_inode.children.push((name.to_string(), inode_index));
        (self.stat(parent_dir), self.stat(inode_index))
    }

    fn unlink(&mut self, parent_inode: u32, name: &str) {
        if let Some(parent_node) = self.inodes.get_mut(&parent_inode) {
            parent_node.children.retain(|(n, _)| n != name);
        }
    }

    fn link(&mut self, _inode: u32, _parent_dir: u32, _name: &str) -> super::Inode {
        panic!("Linking is not supported in dtmpfs");
    }

    fn truncate(&mut self, _inode: u32, _size: u64) {
        panic!("Truncating is not supported in dtmpfs");
    }

    fn rename(&mut self, inode: u32, parent_inode: u32, name: &str) {
        let Some(parent_node) = self.inodes.get_mut(&parent_inode) else {
            return;
        };
        if let Some((_, child_inode)) = parent_node.children.iter_mut().find(|(n, _)| n == name) {
            *child_inode = inode;
        } else {
            parent_node.children.push((name.to_string(), inode));
        }
    }
}
