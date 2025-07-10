use std::{boxed::Box, mem_utils::PhysAddr};

use crate::drivers::disk::{DirEntry, MountedPartition};

use super::{Inode, InodeIndex, InodeType};


pub trait FileSystemFactory {
    fn mount(&self, partition: MountedPartition) -> Box<dyn FileSystem + Send>;
}

pub trait FileSystem {
    fn unmount(&mut self);
    ///Offset must be page aligned
    fn read(&mut self, inode: InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]);
    fn read_dir(&mut self, inode: InodeIndex) -> Box<[DirEntry]>;
    ///Offset must be page aligned. Returns the new inode
    fn write(&mut self, inode: InodeIndex, offset: u64, size: u64, buffer: &[PhysAddr]) -> Inode;
    fn stat(&mut self, inode: InodeIndex) -> Inode;
    fn set_stat(&mut self, inode_index: InodeIndex, inode_data: Inode);
    ///returns the new parent inode in the first field and the new inode in the second
    fn create(&mut self, name: &str, parent_dir: InodeIndex, type_mode: InodeType, uid: u16, gid: u16) -> (Inode, Inode);
    fn unlink(&mut self, parent_inode: InodeIndex, name: &str);
    ///returns the new parent inode
    fn link(&mut self, inode: InodeIndex, parent_dir: InodeIndex, name: &str) -> Inode;
    fn truncate(&mut self, inode: InodeIndex, size: u64);
    fn rename(&mut self, inode: InodeIndex, parent_inode: InodeIndex, name: &str);
}
