use std::{boxed::Box, mem_utils::VirtAddr, string::String, vec::Vec, PAGE_ALLOCATOR};

use crate::disk::Partition;

use super::PartitionSchemeDriver;

pub struct GPTDriver {}

impl PartitionSchemeDriver for GPTDriver {
    fn partitions(&self, disk: &mut dyn super::Disk) -> Vec<(u128, super::Partition)> {
        let first_lba = Box::new([0u8; 512]);
        let first_lba_ptr = &*first_lba as *const [u8; 512] as *const u8;
        let command_slot = disk.read(1, 1, VirtAddr(first_lba_ptr as u64));
        disk.clean_after_read(command_slot);
        let header: &GptHeader = unsafe { &*(first_lba_ptr as *const GptHeader) };

        assert_eq!(header.signature, *b"EFI PART", "Not a GPT disk");

        let start_entries = header.partition_entry_lba as usize;
        let num_entries = header.num_partition_entries as usize;
        let entry_size = header.size_partition_entry as usize;
        let entry_num_lbas = (num_entries * entry_size + 511) / 512;
        let buffer = unsafe { PAGE_ALLOCATOR.allocate_contigious(entry_num_lbas as u64 / 8, None) };
        let command_slot = disk.read(start_entries, entry_num_lbas, buffer);
        disk.clean_after_read(command_slot);

        let mut partitions = Vec::new();

        for i in 0..num_entries {
            unsafe {
                let ptr = (buffer.0 as *const u8).add(i * entry_size);
                let entry = ptr as *const GptEntry;
                let entry = entry.read_volatile();
                if entry.partition_type_guid == [0; 16] {
                    continue;
                }
                let mut name = String::from_utf16(&entry.partition_name).unwrap();
                name.remove_matches("\u{0}");
                let guid = u128::from_le_bytes(entry.unique_partition_guid);
                partitions.push((
                    guid,
                    Partition {
                        start_sector: entry.starting_lba as usize,
                        size_sectors: (entry.ending_lba - entry.starting_lba + 1) as usize,
                        name,
                        disk: self.guid(disk),
                    },
                ))
            }
        }

        unsafe { 
            //free memory
            for i in 0..(entry_num_lbas / 8) {
                PAGE_ALLOCATOR.deallocate(buffer + VirtAddr(i as u64 * 4096));
            }
        }

        partitions
    }

    fn guid(&self, disk: &mut dyn super::Disk) -> u128 {
        let first_lba = Box::new([0u8; 512]);
        let first_lba_ptr = &*first_lba as *const [u8; 512];
        let command_slot = disk.read(1, 1, VirtAddr(first_lba_ptr as u64));
        disk.clean_after_read(command_slot);
        let header: &GptHeader = unsafe { &*(first_lba_ptr as *const GptHeader) };
        u128::from_le_bytes(header.disk_guid)
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

#[derive(Debug)]
#[repr(C)]
pub struct GptEntry {
    partition_type_guid: [u8; 16],
    unique_partition_guid: [u8; 16],
    starting_lba: u64,
    ending_lba: u64,
    attributes: u64,
    partition_name: [u16; 36],
}
