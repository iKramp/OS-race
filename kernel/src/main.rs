#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]
#![feature(box_into_inner)]
#![feature(string_remove_matches)]
#![feature(arbitrary_self_types)]
#![feature(arbitrary_self_types_pointers)]
#![feature(c_str_module)]
#![feature(str_from_raw_parts)]
#![feature(slice_index_methods)]
#![feature(new_range_api)]

extern crate static_cond;

use core::{ffi, time::Duration};
use std::{println, printlnc};

mod acpi;
mod clocks;
mod cmd_args;
mod cpuid;
mod drivers;
mod file_operations;
mod interrupts;
mod keyboard;
mod limine;
mod memory;
mod msr;
mod parsers;
mod pci;
mod proc;
#[allow(unused_imports)]
mod tests;
mod utils;
mod vfs;
mod vga;
use limine::LIMINE_BOOTLOADER_REQUESTS;
use vfs::ResolvedPath;

const TEST_EXECUTABLE: &[u8] = include_bytes!("../../assets/libr");

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    let stack_pointer: *const u64;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) stack_pointer);
    }
    vga::init_vga_driver();
    vga::clear_screen();

    let cmd_line_info = unsafe { &(*LIMINE_BOOTLOADER_REQUESTS.cmd_line_request.info) };
    let str = unsafe { ffi::CStr::from_ptr(cmd_line_info.cmdline) };

    println!("starting RustOs...");
    println!("stack pointer: {:?}", stack_pointer);

    interrupts::init_interrupts();

    memory::init_memory();

    let cmd_args = cmd_args::CmdArgs::new(str.to_str().unwrap());
    println!("cmd_args: {:?}", cmd_args);

    acpi::read_tables();

    clocks::init();

    acpi::init_acpi();

    pci::enumerate_devices();
    vfs::init();

    let res = vfs::mount_blkdev_partition(cmd_args.root_partition, ResolvedPath::root());
    if let Err(e) = res {
        println!("{}", e);
        panic!("Failed to mount root partition");
    }

    let path = vfs::resolve_path("/", "/");
    println!("{:?}", vfs::get_dir_entries((&path).into()));

    proc::init();

    // file_operations::do_file_operations();

    // vga_text::hello_message();

    #[cfg(feature = "run_tests")]
    {
        println!("Running tests");
        use tests::test_runner;
        test_runner();
        println!("Finished running tests");
    }

    //start first proc
    // unsafe { core::arch::asm!("int 254") };

    println!("looping infinitely now");
    let mut a = 0;
    #[allow(clippy::empty_loop)]
    loop {
        a += 1;
        acpi::set_timeout(Duration::from_secs(1));
        println!("a: {}", a);
        unsafe { core::arch::asm!("hlt") };
    }
}
