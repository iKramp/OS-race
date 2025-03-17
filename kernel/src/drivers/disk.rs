use core::fmt::Debug;
use std::{boxed::Box, mem_utils::{PhysAddr, VirtAddr}, string::String, vec::Vec};

use crate::vfs::{Inode, InodeType};



pub trait Disk: Debug {
    fn read(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64;
    fn write(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64;
    fn clean_after_read(&mut self, metadata: u64);
    fn clean_after_write(&mut self, metadata: u64);
}

pub trait PartitionSchemeDriver {
    fn guid(&self, disk: &mut dyn Disk) -> u128;
    fn partitions(&self, disk: &mut dyn Disk) -> Vec<(u128, Partition)>;
}

pub trait FileSystemFactory {
    fn guid(&self) -> u128;
    fn mount(&self, partition: MountedPartition) -> Box<dyn FileSystem>;
}

pub trait FileSystem {
    fn unmount(&mut self);
    ///Offset must be page aligned
    fn read(&mut self, inode: u32, offset: u64, size: u64, buffer: &[PhysAddr]);
    ///Offset must be page aligned
    fn write(&mut self, inode: u32, offset: u64, size: u64, buffer: &[PhysAddr]);
    fn stat(&mut self, inode: u32) -> Inode;
    fn set_stat(&mut self, inode_index: u32, inode_data: Inode);
    fn create(&mut self, name: String, parent_dir: u32, type_mode: InodeType, uid: u16, gid: u16) -> Inode;
    fn remove(&mut self, inode: u32);
    fn link(&mut self, inode: u32, parent_dir: u32, name: String);
    fn truncate(&mut self, inode: u32, size: u64);
    fn rename(&mut self, inode: u32, parent_inode: u32, name: String);
}

#[derive(Debug)]
pub struct MountedPartition {
    pub disk: &'static mut dyn Disk,
    pub partition: Partition,
}

#[derive(Debug)]
pub struct Partition {
    pub disk: u128,
    pub start_sector: usize,
    pub size_sectors: usize,
    pub name: String,
}

impl MountedPartition {
    pub fn new(disk: &'static mut dyn Disk, partition: Partition) -> Self {
        Self {
            disk,
            partition,

        }
    }

    pub fn read(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64 {
        assert!(sector + sec_count <= self.partition.size_sectors);
        self.disk.read(self.partition.start_sector + sector, sec_count, buffer)
    }

    pub fn write(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64 {
        assert!(sector + sec_count <= self.partition.size_sectors);
        self.disk.write(self.partition.start_sector + sector, sec_count, buffer)
    }

    pub fn clean_after_read(&mut self, metadata: u64) {
        self.disk.clean_after_read(metadata);
    }

    pub fn clean_after_write(&mut self, metadata: u64) {
        self.disk.clean_after_write(metadata);
    }
}
