use core::fmt::Debug;
use std::{boxed::Box, mem_utils::VirtAddr, string::String, vec::Vec};



pub trait Disk: Debug {
    fn read(&mut self, sector: usize, sec_count: usize, buffer: VirtAddr) -> u64;
    fn write(&mut self, sector: usize, sec_count: usize, buffer: VirtAddr) -> u64;
    fn clean_after_read(&mut self, metadata: u64);
    fn clean_after_write(&mut self, metadata: u64);
}

pub trait PartitionSchemeDriver {
    fn guid(&self, disk: &mut dyn Disk) -> u128;
    fn partitions(&self, disk: &mut dyn Disk) -> Vec<(u128, Partition)>;
}

pub trait FileSystemFactory {
    fn guid(&self) -> u128;
    fn create(&self, disk: &'static mut dyn Disk) -> Box<dyn FileSystem>;
}

pub trait FileSystem {
    //TODO:
}

#[derive(Debug)]
pub struct Partition {
    pub start_sector: usize,
    pub size_sectors: usize,
    pub name: String,
    pub disk: u128,
}
