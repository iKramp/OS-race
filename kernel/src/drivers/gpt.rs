use std::{boxed::Box, mem_utils::VirtAddr, println, vec::Vec};

use super::PartitionSchemeDriver;


pub struct GPTDriver {
}

impl PartitionSchemeDriver for GPTDriver {
    fn partitions(&self, disk: &mut dyn super::DiskDriver) -> Vec<super::Partition> {
        //Vec::new();
        
        let first_lba: *const [u8; 512] = Box::leak(Box::new([0u8; 512]));
        let command_slot = disk.read(1, 1, VirtAddr(first_lba as u64));
        disk.clean_after_read(command_slot);
        let header: &GptHeader = unsafe { &*(first_lba as *const GptHeader) };
        println!("GPT Header: {:#x?}", header);





        todo!();
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct GptHeader {
    signature: [u8; 8],
    revision: u32,
    header_size: u32,
    header_crc32: u32,
    reserved: u32,
    this_lba: u64,
    alternate_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    disk_guid: [u8; 16],
    partition_entry_lba: u64,
    num_partition_entries: u32,
    size_partition_entry: u32,
    partition_entry_array_crc32: u32,
}
