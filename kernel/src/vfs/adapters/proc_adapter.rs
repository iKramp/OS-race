use super::VfsAdapterTrait;


pub(super) struct ProcAdapter;

impl VfsAdapterTrait for ProcAdapter {
    fn read(&mut self, inode: crate::vfs::InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[std::mem_utils::PhysAddr]) {
        todo!()
    }

    fn read_dir(&mut self, inode: crate::vfs::InodeIndex) -> std::boxed::Box<[crate::drivers::disk::DirEntry]> {
        todo!()
    }

    fn write(&mut self, inode: crate::vfs::InodeIndex, offset: u64, size: u64, buffer: &[std::mem_utils::PhysAddr]) -> crate::vfs::Inode {
        todo!()
    }

    fn stat(&mut self, inode: crate::vfs::InodeIndex) -> crate::vfs::Inode {
        todo!()
    }
}
