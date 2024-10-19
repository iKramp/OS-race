#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]

use std::{eh::int3, panic::PanicInfo};

mod acpi;
mod cpuid;
mod interrupts;
mod keyboard;
mod limine;
mod memory;
#[allow(unused_imports)]
mod tests;
mod utils;
mod vga;
use limine::LIMINE_BOOTLOADER_REQUESTS;
use vga::vga_text;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_text::set_vga_text_foreground((0, 0, 255));
    println!("{}", info);
    int3();
    std::panic::print_stack_trace();
    loop {}
}

pub struct BootInfo {}

#[no_mangle]
extern "C" fn _start() -> ! {
    vga::init_vga_driver();
    vga::clear_screen();

    interrupts::init_interrupts();

    std::panic::test_fn_1();

    println!("starting RustOs...");

    let boot_info = unsafe { &*LIMINE_BOOTLOADER_REQUESTS.bootloader_info_request.info };
    let name = utils::ptr_to_str(boot_info.name);
    println!("Booted with bootloader: {:?}", name);
    let version = utils::ptr_to_str(boot_info.version);
    println!("Version: {}", version);

    memory::init_memory();

    acpi::init_acpi();

    //vga_text::hello_message();

    let last_time = crate::interrupts::time_since_boot();
    loop {
        if last_time + std::time::Duration::from_millis(500) < crate::interrupts::time_since_boot() {
            break;
        }
    }

    #[cfg(feature = "run_tests")]
    {
        println!("Running tests");
        use tests::test_runner;
        test_runner();
    }

    #[allow(clippy::empty_loop)]
    loop {}
}
