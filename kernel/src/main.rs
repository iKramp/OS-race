#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]
#![feature(box_into_inner)]
#![feature(string_remove_matches)]
#![feature(arbitrary_self_types)]
#![feature(arbitrary_self_types_pointers)]

use std::{println, printlnc};

mod acpi;
mod cpuid;
mod drivers;
mod interrupts;
mod keyboard;
mod limine;
mod memory;
mod msr;
mod pci;
#[allow(unused_imports)]
mod tests;
mod utils;
mod vfs;
mod vga;
use limine::LIMINE_BOOTLOADER_REQUESTS;
use vga::vga_text;

pub struct BootInfo {}

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    let stack_pointer: *const u64;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) stack_pointer);
    }
    unsafe { std::thread::GET_TIME_SINCE_BOOT = || interrupts::time_since_boot() };
    vga::init_vga_driver();
    vga::clear_screen();

    println!("starting RustOs...");
    println!("stack pointer: {:?}", stack_pointer);

    interrupts::init_interrupts();

    memory::init_memory();

    acpi::init_acpi();

    pci::enumerate_devices();

    vga_text::hello_message();

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
        println!("a: {}", a);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
