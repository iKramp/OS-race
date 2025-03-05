#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]
#![feature(box_into_inner)]
#![feature(string_remove_matches)]
#![feature(arbitrary_self_types)]
#![feature(arbitrary_self_types_pointers)]

use std::{print, println, printlnc};

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
mod pci;
mod drivers;
mod vfs;
use drivers::{rfs::{Rfs, RfsFactory}, virtual_disk::VirtualDisk};
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

    //vga_text::hello_message();
    //
    //#[cfg(feature = "run_tests")]
    //{
    //    println!("Running tests");
    //    use tests::test_runner;
    //    test_runner();
    //    println!("Finished running tests");
    //}

    let mut virt_disk = VirtualDisk {
        data: Default::default(),
    };
    let virt_disk: &'static mut VirtualDisk = unsafe {&mut *((&mut virt_disk) as *mut VirtualDisk)};
    let mut filesystem = Rfs::new(virt_disk);
    //for i in 0..117700 {
    //    filesystem.add_key(i, i);
    //} 
    //
    //for i in 0..117700 {
    //    filesystem.remove_key(i);
    //}
    
    for i in 3..10000 {
        filesystem.add_key(i, i);
    }
    for i in 3..10000 {
        filesystem.remove_key(i);
    }

    println!("looping infinitely now");
    let mut a = 2;
    #[allow(clippy::empty_loop)]
    loop {
        a += 100;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
