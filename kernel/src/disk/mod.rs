use std::{boxed::Box, collections::btree_map::BTreeMap, println, string::String};

use crate::drivers::{gpt::GPTDriver, Disk, PartitionSchemeDriver};


static mut DISKS: BTreeMap<u128, Box<dyn Disk>> = BTreeMap::new();
static mut PARTITIONS: BTreeMap<u128, Partition> = BTreeMap::new();

pub fn add_disk(mut disk: Box<dyn Disk>) {
    //for now only GPT
    let gpt_driver = GPTDriver {};
    let guid = gpt_driver.guid(&mut *disk);
    let partitions = gpt_driver.partitions(&mut *disk);

    unsafe {
        DISKS.insert(guid, disk);
        for (guid, partition) in partitions {
            PARTITIONS.insert(guid, partition);
        }
    }
}

pub fn print_disks() {
    unsafe {
        for disk in DISKS.iter() {
            println!("Disk: {:#x?}", disk);
        }
    }
}

pub fn print_partitions() {
    unsafe {
        for partition in PARTITIONS.iter() {
            println!("Partition: {:#x?}", partition);
        }
    }
}

#[derive(Debug)]
pub struct Partition {
    pub start_sector: usize,
    pub size_sectors: usize,
    pub name: String,
    pub disk: u128,
}
