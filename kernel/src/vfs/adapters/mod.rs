use core::fmt::Debug;
use std::{boxed::Box, mem_utils::PhysAddr};

use crate::drivers::disk::DirEntry;

use super::{filesystem_trait::FileSystem, Inode, InodeIndex};

mod proc_adapter;

const DEV_NAMESPACE: u64 = 0b00000000 << 56;
const PROC_NAMESPACE: u64 = 0b00000001 << 56;
const SYS_NAMESPACE: u64 = 0b00000010 << 56;
const RUN_NAMESPACE: u64 = 0b00000011 << 56;
const MASK: u64 = 0b11111111 << 56;

#[derive(Debug)]
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

#[async_trait::async_trait]
impl VfsAdapterTrait for VfsAdpater {
    async fn read(&self, inode: super::InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[std::mem_utils::PhysAddr]) {
        match inode & MASK {
            PROC_NAMESPACE => FileSystem::read(&self.proc, inode, offset_bytes, size_bytes, buffer).await,
            _ => unreachable!(),
        }
    }

    async fn read_dir(&self, inode: super::InodeIndex) -> std::boxed::Box<[crate::drivers::disk::DirEntry]> {
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

    async fn write(&self, inode: super::InodeIndex, offset: u64, size: u64, buffer: &[std::mem_utils::PhysAddr]) -> super::Inode {
        match inode & MASK {
            PROC_NAMESPACE => {
                let mut inode = FileSystem::write(&self.proc, inode & !MASK, offset, size, buffer).await;
                inode.index |= PROC_NAMESPACE;
                inode
            }
            _ => unreachable!(),
        }
    }

    async fn stat(&self, inode: super::InodeIndex) -> super::Inode {
        match inode & MASK {
            PROC_NAMESPACE => FileSystem::stat(&self.proc, inode & !MASK).await,
            _ => unreachable!(),
        }
    }
}

#[async_trait::async_trait]
pub trait VfsAdapterTrait: Debug + Send + Sync {
    async fn read(&self, inode: InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]);
    async fn read_dir(&self, inode: InodeIndex) -> Box<[DirEntry]>;
    async fn write(&self, inode: InodeIndex, offset: u64, size: u64, buffer: &[PhysAddr]) -> Inode;
    async fn stat(&self, inode: InodeIndex) -> Inode;
}

#[async_trait::async_trait]
impl<T: VfsAdapterTrait> FileSystem for T {
    async fn read(&self, inode: InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]) {
        VfsAdapterTrait::read(self, inode, offset_bytes, size_bytes, buffer).await;
    }

    async fn read_dir(&self, inode: InodeIndex) -> Box<[DirEntry]> {
        VfsAdapterTrait::read_dir(self, inode).await
    }

    async fn write(&self, inode: InodeIndex, offset: u64, size: u64, buffer: &[PhysAddr]) -> Inode {
        VfsAdapterTrait::write(self, inode, offset, size, buffer).await
    }

    async fn stat(&self, inode: InodeIndex) -> Inode {
        VfsAdapterTrait::stat(self, inode).await
    }

    async fn unmount(&self) {
        unreachable!()
    }

    async fn set_stat(&self, _inode_index: InodeIndex, _inode_data: Inode) {
        unreachable!()
    }

    async fn create(
        &self,
        _name: &str,
        _parent_dir: InodeIndex,
        _type_mode: super::InodeType,
        _uid: u16,
        _gid: u16,
    ) -> (Inode, Inode) {
        unreachable!()
    }

    async fn unlink(&self, _parent_inode: InodeIndex, _name: &str) {
        unreachable!()
    }

    async fn link(&self, _inode: InodeIndex, _parent_dir: InodeIndex, _name: &str) -> Inode {
        unreachable!()
    }

    async fn truncate(&self, _inode: InodeIndex, _size: u64) {
        unreachable!()
    }

    async fn rename(&self, _inode: InodeIndex, _parent_inode: InodeIndex, _name: &str) {
        unreachable!()
    }
}
