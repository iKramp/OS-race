pub mod paging;
pub mod physical_allocator;
use std::mem_utils;

pub static mut PAGE_TREE_ALLOCATOR: paging::PageTree = paging::PageTree {
    level_4_table: std::mem_utils::PhysAddr(0),
};

pub fn init_memory(boot_info: &'static mut bootloader_api::BootInfo) {
    unsafe {
        let offset: Option<u64> = boot_info.physical_memory_offset.into();
        mem_utils::set_physical_offset(mem_utils::PhysOffset(offset.unwrap()));
        physical_allocator::BuyddyAllocator::init(boot_info);
        PAGE_TREE_ALLOCATOR = paging::PageTree::new();
        std::PAGE_ALLOCATOR = #[allow(static_mut_refs)]
        &mut PAGE_TREE_ALLOCATOR;
    }
}
