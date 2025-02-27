use core::fmt::Debug;
use std::{mem_utils::VirtAddr, string::String, vec::Vec};

use crate::disk::Partition;

pub mod ahci;
pub mod gpt;

pub trait PCIDriver: Debug {
    fn class(&self) -> super::pci::device_config::PciClass;
    fn vendor_id(&self) -> Option<u16> {
        None
    }
    fn device_id(&self) -> Option<u16> {
        None
    }
}

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
