use core::fmt::Debug;
use std::{boxed::Box, mem_utils::PhysAddr, string::String, vec::Vec};

use uuid::Uuid;

use crate::vfs::{DeviceId, InodeIndex};

pub trait BlockDevice: Debug {
    fn read(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64;
    fn write(&mut self, sector: usize, sec_count: usize, buffer: &[PhysAddr]) -> u64;
    fn clean_after_read(&mut self, metadata: u64);
    fn clean_after_write(&mut self, metadata: u64);
}

pub trait PartitionSchemeDriver {
    fn guid(&self, disk: &mut dyn BlockDevice) -> Uuid;
    ///returns a vector of partition guids (not filesystem ids) and partition objects
    fn partitions(&self, disk: &mut dyn BlockDevice) -> Vec<(Uuid, Partition)>;
}

#[derive(Debug)]
pub struct MountedPartition {
    pub disk: &'static mut dyn BlockDevice,
    pub partition: Partition,
}

#[derive(Debug, Clone)]
pub struct Partition {
    pub fs_type: Uuid,
    pub device: DeviceId,
    pub start_sector: usize,
    pub size_sectors: usize,
    pub name: String,
}

impl MountedPartition {
    pub fn new(disk: &'static mut dyn BlockDevice, partition: Partition) -> Self {
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
    pub inode: InodeIndex,
    pub name: Box<str>,
}
