use std::{boxed::Box, mem_utils::PhysAddr};

use crate::drivers::disk::DirEntry;

use super::{filesystem_trait::FileSystem, Inode, InodeIndex};

mod proc_adapter;

const DEV_NAMESPACE: u64 = 0b00000000 << 56;
const PROC_NAMESPACE: u64 = 0b00000001 << 56;
const SYS_NAMESPACE: u64 = 0b00000010 << 56;
const RUN_NAMESPACE: u64 = 0b00000011 << 56;
const MASK: u64 = 0b11111111 << 56;

struct VfsAdpater {
    proc: proc_adapter::ProcAdapter,
}

impl VfsAdpater {
    pub fn new() -> Self {
        VfsAdpater {
            proc: proc_adapter::ProcAdapter,
        }
    }
}

impl VfsAdapterTrait for VfsAdpater {
    fn read(&mut self, inode: super::InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[std::mem_utils::PhysAddr]) {
        match inode & MASK {
            PROC_NAMESPACE => FileSystem::read(&mut self.proc, inode, offset_bytes, size_bytes, buffer),
            _ => unreachable!(),
        }
    }

    fn read_dir(&mut self, inode: super::InodeIndex) -> std::boxed::Box<[crate::drivers::disk::DirEntry]> {
        match inode & MASK {
            PROC_NAMESPACE => {
                // let mut dir_entries = FileSystem::read_dir(&mut self.proc, inode);
                // dir_entries = dir_entries.iter_mut().map(|entry| {
                //     let mut new_entry = entry.clone();
                //     new_entry.inode.index |= PROC_NAMESPACE;
                //     new_entry
                // }).collect();
                unreachable!()
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, inode: super::InodeIndex, offset: u64, size: u64, buffer: &[std::mem_utils::PhysAddr]) -> super::Inode {
        match inode & MASK {
            PROC_NAMESPACE => {
                let mut inode = FileSystem::write(&mut self.proc, inode & !MASK, offset, size, buffer);
                inode.index |= PROC_NAMESPACE;
                inode
            }
            _ => unreachable!(),
        }
    }

    fn stat(&mut self, inode: super::InodeIndex) -> super::Inode {
        match inode & MASK {
            PROC_NAMESPACE => FileSystem::stat(&mut self.proc, inode & !MASK),
            _ => unreachable!(),
        }
    }
}

pub trait VfsAdapterTrait {
    fn read(&mut self, inode: InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]);
    fn read_dir(&mut self, inode: InodeIndex) -> Box<[DirEntry]>;
    fn write(&mut self, inode: InodeIndex, offset: u64, size: u64, buffer: &[PhysAddr]) -> Inode;
    fn stat(&mut self, inode: InodeIndex) -> Inode;
}

impl<T: VfsAdapterTrait> FileSystem for T {
    fn read(&mut self, inode: InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]) {
        VfsAdapterTrait::read(self, inode, offset_bytes, size_bytes, buffer);
    }

    fn read_dir(&mut self, inode: InodeIndex) -> Box<[DirEntry]> {
        VfsAdapterTrait::read_dir(self, inode)
    }

    fn write(&mut self, inode: InodeIndex, offset: u64, size: u64, buffer: &[PhysAddr]) -> Inode {
        VfsAdapterTrait::write(self, inode, offset, size, buffer)
    }

    fn stat(&mut self, inode: InodeIndex) -> Inode {
        VfsAdapterTrait::stat(self, inode)
    }

    fn unmount(&mut self) {
        unreachable!()
    }

    fn set_stat(&mut self, _inode_index: InodeIndex, _inode_data: Inode) {
        unreachable!()
    }

    fn create(
        &mut self,
        _name: &str,
        _parent_dir: InodeIndex,
        _type_mode: super::InodeType,
        _uid: u16,
        _gid: u16,
    ) -> (Inode, Inode) {
        unreachable!()
    }

    fn unlink(&mut self, _parent_inode: InodeIndex, _name: &str) {
        unreachable!()
    }

    fn link(&mut self, _inode: InodeIndex, _parent_dir: InodeIndex, _name: &str) -> Inode {
        unreachable!()
    }

    fn truncate(&mut self, _inode: InodeIndex, _size: u64) {
        unreachable!()
    }

    fn rename(&mut self, _inode: InodeIndex, _parent_inode: InodeIndex, _name: &str) {
        unreachable!()
    }
}
