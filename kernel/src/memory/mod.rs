pub mod paging;
pub mod physical_allocator;
use crate::println;
use std::mem_utils::{self, VirtAddr};

pub static mut PAGE_TREE_ALLOCATOR: paging::PageTree = paging::PageTree {
    level_4_table: std::mem_utils::PhysAddr(0),
};

pub fn init_memory(/*boot_info: &mut bootloader_api::BootInfo*/) {
    println!("initializing memory");
    /*unsafe {
        let offset: Option<u64> = boot_info.physical_memory_offset.into();
        mem_utils::set_physical_offset(mem_utils::PhysOffset(offset.unwrap()));
        let boot_info_ptr = boot_info as *mut bootloader_api::BootInfo;
        println!("initializing physical allocator");
        physical_allocator::BuyddyAllocator::init(&mut *boot_info_ptr);
        println!("initializing pager");
        PAGE_TREE_ALLOCATOR = paging::PageTree::new();
        crate::vga_text::set_vga_text_foreground((0, 255, 0));
        println!("memory initialized");
        crate::vga_text::reset_vga_color();
        #[allow(static_mut_refs)]
        {
            std::PAGE_ALLOCATOR = &mut PAGE_TREE_ALLOCATOR;
        }
        let page_table_entry =
            PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(VirtAddr(crate::vga::vga_driver::VGA_BINDING.buffer as u64));
        page_table_entry.set_disable_cahce(true);
        page_table_entry.set_write_through_cahcing(true);
        core::arch::asm!(
            "mov rax, cr3",
            "mov cr3, rax",
            out("rax") _
        ); //clear the TLB
    }*/
}
