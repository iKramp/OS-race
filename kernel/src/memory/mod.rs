mod paging;
mod physical_allocator;
pub mod utils;

pub fn init_memory(boot_info: &'static mut bootloader_api::BootInfo) {
    unsafe {
        let offset: Option<u64> = boot_info.physical_memory_offset.into();
        utils::set_physical_offset(utils::PhysOffset(offset.unwrap()));
    }
    physical_allocator::BuyddyAllocator::init(boot_info);
    let virtual_allocator = paging::PageTree::new();
}
