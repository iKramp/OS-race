use core::fmt::Debug;
use std::{boxed::Box, mem_utils::PhysAddr, string::String, vec::Vec};

use uuid::Uuid;

use crate::vfs::{DeviceId, Inode, InodeType};

pub trait Disk: Debug {
    fn read(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64;
    fn write(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64;
    fn clean_after_read(&mut self, metadata: u64);
    fn clean_after_write(&mut self, metadata: u64);
}

pub trait PartitionSchemeDriver {
    fn guid(&self, disk: &mut dyn Disk) -> Uuid;
    ///returns a vector of partition guids (not filesystem ids) and partition objects
    fn partitions(&self, disk: &mut dyn Disk) -> Vec<(Uuid, Partition)>;
}

pub trait FileSystemFactory {
    fn mount(&self, partition: MountedPartition) -> Box<dyn FileSystem + Send>;
}

pub trait FileSystem {
    fn unmount(&mut self);
    ///Offset must be page aligned
    fn read(&mut self, inode: u32, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]);
    fn read_dir(&mut self, inode: u32) -> Box<[DirEntry]>;
    ///Offset must be page aligned. Returns the new inode
    fn write(&mut self, inode: u32, offset: u64, size: u64, buffer: &[PhysAddr]) -> Inode;
    fn stat(&mut self, inode: u32) -> Inode;
    fn set_stat(&mut self, inode_index: u32, inode_data: Inode);
    ///returns the new parent inode in the first field and the new inode in the second
    fn create(&mut self, name: &str, parent_dir: u32, type_mode: InodeType, uid: u16, gid: u16) -> (Inode, Inode);
    fn unlink(&mut self, parent_inode: u32, name: &str);
    ///returns the new parent inode
    fn link(&mut self, inode: u32, parent_dir: u32, name: &str) -> Inode;
    fn truncate(&mut self, inode: u32, size: u64);
    fn rename(&mut self, inode: u32, parent_inode: u32, name: &str);
}

#[derive(Debug)]
pub struct MountedPartition {
    pub disk: &'static mut dyn Disk,
    pub partition: Partition,
}

#[derive(Debug, Clone)]
pub struct Partition {
    pub fs_uuid: Uuid,
    pub device: DeviceId,
    pub start_sector: usize,
    pub size_sectors: usize,
    pub name: String,
}

impl MountedPartition {
    pub fn new(disk: &'static mut dyn Disk, partition: Partition) -> Self {
        Self { disk, partition }
    }

    pub fn read(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) {
        assert!(sector + sec_count <= self.partition.size_sectors);
        let metadata = self.disk.read(self.partition.start_sector + sector, sec_count, buffer);
        self.disk.clean_after_read(metadata);
    }

    pub fn write(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) {
        assert!(sector + sec_count <= self.partition.size_sectors);
        let metadata = self.disk.write(self.partition.start_sector + sector, sec_count, buffer);
        self.disk.clean_after_write(metadata);
    }
}

#[derive(Debug)]
pub struct DirEntry {
    pub inode: u32,
    pub name: Box<str>,
}
