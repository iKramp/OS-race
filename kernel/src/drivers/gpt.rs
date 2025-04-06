use std::{
    PAGE_ALLOCATOR, PageAllocator,
    mem_utils::{PhysAddr, get_at_virtual_addr, translate_virt_phys_addr},
    println,
    string::String,
    vec::Vec,
};

use uuid::Uuid;

use crate::{memory::{paging::{LiminePat, PageTree}, physical_allocator, PAGE_TREE_ALLOCATOR}, vfs::VFS};

use super::disk::{Disk, Partition, PartitionSchemeDriver};

pub struct GPTDriver {}

impl PartitionSchemeDriver for GPTDriver {
    fn partitions(&self, disk: &mut dyn Disk) -> Vec<(Uuid, Partition)> {
        println!("GPT partitions");
        let first_lba = physical_allocator::allocate_frame();
        let first_lba_binding = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(first_lba)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(first_lba_binding)
                .set_pat(LiminePat::UC);
        }
        let command_slot = disk.read(1, 1, &[first_lba]);
        disk.clean_after_read(command_slot);
        let header = unsafe { get_at_virtual_addr::<GptHeader>(first_lba_binding) };

        assert_eq!(header.signature, *b"EFI PART", "Not a GPT disk");

        let start_entries = header.partition_entry_lba as usize;
        let num_entries = header.num_partition_entries as usize;
        let entry_size = header.size_partition_entry as usize;
        let entry_num_lbas = (num_entries * entry_size).div_ceil(512);
        let buffer = unsafe { PAGE_ALLOCATOR.allocate_contigious(entry_num_lbas as u64 / 8, None) };
        let page_tree_root = PageTree::get_level4_addr();
        let physical_addresses: Vec<PhysAddr> = (0..entry_num_lbas / 8)
            .inspect(|i| unsafe {
                PAGE_TREE_ALLOCATOR
                    .get_page_table_entry_mut(buffer + (*i as u64 * 4096))
                    .set_pat(LiminePat::UC)
            })
            .map(|i| translate_virt_phys_addr(buffer + (i as u64 * 4096), page_tree_root).unwrap())
            .collect();
        let command_slot = disk.read(start_entries, entry_num_lbas, &physical_addresses);
        disk.clean_after_read(command_slot);

        let mut partitions = Vec::new();

        let mut vfs = VFS.lock();

        for i in 0..num_entries {
            unsafe {
                let ptr = (buffer.0 as *mut u8).add(i * entry_size);
                let entry_ptr = ptr as *mut GptEntry;
                let entry = entry_ptr.read_volatile();
                if entry.partition_type_guid == [0; 16] {
                    continue;
                }
                let mut name = String::from_utf16(&entry.partition_name).unwrap();
                name.remove_matches("\u{0}");
                let partition_uuid = Uuid::from_bytes(entry.unique_partition_guid);
                let fs_uuid = Uuid::from_bytes(entry.partition_type_guid);
                partitions.push((
                    partition_uuid,
                    Partition {
                        start_sector: entry.starting_lba as usize,
                        size_sectors: (entry.ending_lba - entry.starting_lba + 1) as usize,
                        name,
                        device: vfs.allocate_device(),
                        fs_uuid,
                    },
                ))
            }
        }

        unsafe {
            //free memory
            for i in 0..(entry_num_lbas / 8) {
                PAGE_ALLOCATOR.deallocate(buffer + (i as u64 * 4096));
            }
            PAGE_ALLOCATOR.deallocate(first_lba_binding);
        }

        println!("Partitions: {:#?}", partitions);

        partitions
    }

    fn guid(&self, disk: &mut dyn Disk) -> Uuid {
        let first_lba = physical_allocator::allocate_frame();
        let first_lba_binding = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(first_lba)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(first_lba_binding)
                .set_pat(LiminePat::UC);
        }
        let command_slot = disk.read(1, 1, &[first_lba]);
        disk.clean_after_read(command_slot);
        let header = unsafe { get_at_virtual_addr::<GptHeader>(first_lba_binding) };
        let guid = header.disk_guid;
        unsafe { PAGE_ALLOCATOR.deallocate(first_lba_binding) };
        Uuid::from_bytes(guid)
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
