#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]

use std::{panic::PanicInfo, println, printlnc};

mod acpi;
mod cpuid;
mod interrupts;
mod keyboard;
mod limine;
mod memory;
mod msr;
#[allow(unused_imports)]
mod tests;
mod utils;
mod vga;
use limine::LIMINE_BOOTLOADER_REQUESTS;
use vga::vga_text;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printlnc!((0, 0, 255), "{}", info);
    //std::panic::print_stack_trace();
    loop {}
}

pub struct BootInfo {}

#[no_mangle]
extern "C" fn _start() -> ! {
    unsafe { std::thread::GET_TIME_SINCE_BOOT = || interrupts::time_since_boot() };
    vga::init_vga_driver();
    vga::clear_screen();

    println!("starting RustOs...");

    interrupts::init_interrupts();

    memory::init_memory();

    acpi::init_acpi();

    vga_text::hello_message();

    let last_time = crate::interrupts::time_since_boot();
    loop {
        if last_time + std::time::Duration::from_millis(1) < crate::interrupts::time_since_boot() {
            break;
        }
    }

    #[cfg(feature = "run_tests")]
    {
        println!("Running tests");
        use tests::test_runner;
        test_runner();
        println!("Finished running tests");
    }

    println!("looping infinitely now");
    let mut a = 0;
    #[allow(clippy::empty_loop)]
    loop {
        a += 1;
        println!("{}", a);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
