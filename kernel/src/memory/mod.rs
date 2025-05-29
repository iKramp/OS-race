pub mod heap;
pub mod paging;
pub mod physical_allocator;
pub mod stack;

use crate::LIMINE_BOOTLOADER_REQUESTS;
use crate::{println, printlnc};
use std::mem_utils::{self, PhysAddr};

pub static mut PAGE_TREE_ALLOCATOR: paging::PageTree = paging::PageTree::new(PhysAddr(0));

pub static mut TRAMPOLINE_RESERVED: PhysAddr = PhysAddr(0);

pub fn init_memory() {
    println!("initializing memory");
    print_limine_phys_map();
    unsafe {
        let offset: u64 = (*LIMINE_BOOTLOADER_REQUESTS.higher_half_direct_map_request.info).offset;
        mem_utils::set_physical_offset(mem_utils::PhysOffset(offset));
        println!("offset: {:#x?}", offset);
        println!("initializing physical allocator");
        physical_allocator::init();
        //allocates low addresses first, so we reserve this for the trampoline
        TRAMPOLINE_RESERVED = physical_allocator::allocate_frame_low();
        println!("initializing pager");
        let page_table_root = paging::PageTree::get_level4_addr();
        PAGE_TREE_ALLOCATOR = paging::PageTree::new(page_table_root);
        printlnc!((255, 200, 100), "Limine mem map:");
        PAGE_TREE_ALLOCATOR.print_mapping();
        PAGE_TREE_ALLOCATOR.init();
        printlnc!((0, 255, 0), "memory initialized");
    }
}

pub fn print_limine_phys_map() {
    //print limine mmap feature. IS it actually a map?
    printlnc!((255, 200, 100), "Limine physical memory map:");
    let mmap = unsafe { &(*LIMINE_BOOTLOADER_REQUESTS.memory_map_request.info) };
    let entries = unsafe { core::slice::from_raw_parts(mmap.memory_map, mmap.memory_map_count as usize) };
    for entry in entries {
        let start = entry.base;
        let end = entry.base + entry.length;
        let mem_type = match entry.entry_type {
            0 => "Usable",
            1 => "Reserved",
            2 => "ACPI Reclaimable",
            3 => "ACPI NVS",
            4 => "Bad Memory",
            5 => "Bootloader Reclaimable",
            6 => "Kernel and Modules",
            7 => "Framebuffer",
            _ => "Unknown",
        };
        println!("{:#x?} - {:#x?} ({})", start, end, mem_type);
    }
}
