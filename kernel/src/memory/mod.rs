pub mod crate_pagetree;
pub mod paging;
pub mod physical_allocator;

use paging::LiminePat;

use crate::LIMINE_BOOTLOADER_REQUESTS;
use crate::{println, printlnc};
use std::mem_utils::{self, PhysAddr, VirtAddr};

pub static mut PAGE_TREE_ALLOCATOR: paging::PageTree = paging::PageTree {
    level_4_table: std::mem_utils::PhysAddr(0),
};

pub static mut TRAMPOLINE_RESERVED: PhysAddr = PhysAddr(0);

pub fn init_memory() {
    println!("initializing memory");
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
        PAGE_TREE_ALLOCATOR.print_mapping();
        PAGE_TREE_ALLOCATOR.init();
        println!("--------------------------");
        PAGE_TREE_ALLOCATOR.print_mapping();
        printlnc!((0, 255, 0), "memory initialized");
        #[allow(static_mut_refs)]
        {
            std::PAGE_ALLOCATOR = &mut PAGE_TREE_ALLOCATOR;
        }

        print_state();

        //print buffer
        let buffer = crate::vga::vga_driver::VGA_BINDING.buffer as u64;
        println!("VGA buffer: {:#x?}", buffer);
        let page_table_entry =
            PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(VirtAddr(buffer)).unwrap()
;
        page_table_entry.set_pat(LiminePat::UC);
    }
}

pub fn print_state() {
    //print limine mmap feature. IS it actually a map?
    printlnc!((255, 200, 100), "Limine memory map:");
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
        println!(
            "{:#x?} - {:#x?} ({})",
            start, end, mem_type
        );
    }

    physical_allocator::print_state();

    let allocator = unsafe { &PAGE_TREE_ALLOCATOR };
    printlnc!((255, 200, 100), "Memory mapper state:");
    println!("Mapped pages: {}", allocator.get_num_allocated_pages());
}
