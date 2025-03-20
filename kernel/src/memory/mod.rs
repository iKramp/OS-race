pub mod paging;
pub mod physical_allocator;

use paging::LiminePat;

use crate::{println, printlnc};
use std::mem_utils::{self, PhysAddr, VirtAddr};
use crate::LIMINE_BOOTLOADER_REQUESTS;

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
        PAGE_TREE_ALLOCATOR = paging::PageTree::new();
        printlnc!((0, 255, 0), "memory initialized");
        #[allow(static_mut_refs)]
        {
            std::PAGE_ALLOCATOR = &mut PAGE_TREE_ALLOCATOR;
        }
        let page_table_entry =
            PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(VirtAddr(crate::vga::vga_driver::VGA_BINDING.buffer as u64));
        page_table_entry.set_pat(LiminePat::UC);
        core::arch::asm!(
            "mov rax, cr3",
            "mov cr3, rax",
            out("rax") _
        ); //clear the TLB
    }
}
