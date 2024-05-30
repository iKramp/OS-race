#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]

use bootloader_api::{config::Mapping, entry_point, BootloaderConfig};
use core::panic::PanicInfo;

mod interrupts;
mod memory;
#[allow(unused_imports)]
mod tests;
mod vga;
use vga::vga_text;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_text::set_vga_text_foreground((0, 0, 255));
    println!("{}", info);
    loop {}
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

#[no_mangle]
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    unsafe {
        let offset: Option<u64> = boot_info.physical_memory_offset.into();
        memory::utils::set_physical_offset(memory::utils::PhysOffset(offset.unwrap()));
    }

    let binding = boot_info.framebuffer.as_mut().unwrap();
    vga::init_vga_driver(binding);
    vga::clear_screen();

    interrupts::init_interrupts(boot_info.rsdp_addr.into());

    memory::physical_allocator::BuyddyAllocator::init(boot_info);
    let _virtual_allocator = memory::paging::PageTree::new();

    vga_text::hello_message();

    let run_tests = true;
    if run_tests {
        println!("Running tests");
        use crate::tests::test_runner;
        test_runner();
    }

    println!("This message is created after tests, looping infinitely now");

    #[allow(clippy::empty_loop)]
    loop {}
}
