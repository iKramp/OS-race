#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]

use bootloader_api::{config::Mapping, entry_point, BootloaderConfig};
use std::panic::PanicInfo;

mod acpi;
mod cpuid;
mod interrupts;
mod memory;
mod snake;
#[allow(unused_imports)]
mod tests;
mod utils;
mod vga;
mod keyboard;
use vga::vga_text;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_text::set_vga_text_foreground((0, 0, 255));
    println!("{}", info);
    std::panic::print_stack_trace();
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
    let binding = boot_info.framebuffer.as_mut().unwrap();
    vga::init_vga_driver(binding);
    vga::clear_screen();

    println!("starting RustOs...");

    interrupts::init_interrupts();

    memory::init_memory(boot_info);

    acpi::init_acpi(boot_info.rsdp_addr.into());

    vga_text::hello_message();

    let last_time = crate::interrupts::time_since_boot();
    loop {
        if last_time + std::time::Duration::from_secs(2) < crate::interrupts::time_since_boot() {
            break
        }
    }

    let run_tests = false;
    if run_tests {
        println!("Running tests");
        use crate::tests::test_runner;
        test_runner();
    }

    println!("Starting snake");

    let mut state = snake::init();

    #[allow(clippy::empty_loop)]
    let mut last_time = crate::interrupts::time_since_boot();
    loop {
        if last_time + std::time::Duration::from_millis(200) < crate::interrupts::time_since_boot() {
            last_time = crate::interrupts::time_since_boot();
            snake::tick(&mut state);
        }
    }
}
