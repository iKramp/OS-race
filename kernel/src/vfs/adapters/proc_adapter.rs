use super::VfsAdapterTrait;
use std::boxed::Box;


#[derive(Debug)]
pub(super) struct ProcAdapter;

#[async_trait::async_trait]
impl VfsAdapterTrait for ProcAdapter {
    async fn read(&self, _inode: crate::vfs::InodeIndex, _offset_bytes: u64, _size_bytes: u64, _buffer: &[std::mem_utils::PhysAddr]) -> u64 {
        todo!()
    }

    async fn read_dir(&self, _inode: crate::vfs::InodeIndex) -> std::boxed::Box<[crate::drivers::disk::DirEntry]> {
        todo!()
    }

    async fn write(&self, _inode: crate::vfs::InodeIndex, _offset: u64, _size: u64, _buffer: &[std::mem_utils::PhysAddr]) -> crate::vfs::Inode {
        todo!()
    }

    async fn stat(&self, _inode: crate::vfs::InodeIndex) -> crate::vfs::Inode {
        todo!()
    }
}
