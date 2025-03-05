use core::fmt::Debug;
use std::{boxed::Box, mem_utils::{PhysAddr, VirtAddr}, string::String, vec::Vec};

use crate::vfs::{Inode, InodeType};



pub trait Disk: Debug {
    fn read(&mut self, sector: usize, sec_count: usize, buffer: Vec<PhysAddr>) -> u64;
    fn write(&mut self, sector: usize, sec_count: usize, buffer: Vec<PhysAddr>) -> u64;
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
    fn unmount(&self);
    fn read(&self, inode: u32, offset: u32, size: u32, buffer: Vec<PhysAddr>);
    fn write(&self, inode: u32, offset: u32, size: u32, buffer: Vec<PhysAddr>);
    fn stat(&self, inode: u32) -> Inode;
    fn create(&self, path: String, type_mode: InodeType) -> Inode;
    fn remove(&self, inode: u32);
    fn link(&self, inode: u32, path: String);
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

    pub fn read(&mut self, sector: usize, sec_count: usize, buffer: Vec<PhysAddr>) -> u64 {
        assert!(sector + sec_count <= self.partition.size_sectors);
        self.disk.read(self.partition.start_sector + sector, sec_count, buffer)
    }

    pub fn write(&mut self, sector: usize, sec_count: usize, buffer: Vec<PhysAddr>) -> u64 {
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
