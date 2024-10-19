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
mod snake;
#[allow(unused_imports)]
mod tests;
mod utils;
mod vga;
use limine::LIMINE_BOOTLOADER_REQUESTS;
use tests::test_runner;
use vga::vga_text;

use crate::limine::FramebufferMode;

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

    let kernel_file_info = unsafe { &*(*LIMINE_BOOTLOADER_REQUESTS.limine_kernel_file_request.info).address };


    vga::init_vga_driver();
    vga::clear_screen();

    interrupts::init_interrupts();


    let elf_file = std::eh::elf_parser::ElfFile::new(std::mem_utils::VirtAddr(kernel_file_info.address as u64));

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
            break
        }
    }

    let run_tests = false;
    if run_tests {
        println!("Running tests");
        test_runner();
    }

    println!("Starting snake");

    let mut state = snake::init();

    #[allow(clippy::empty_loop)]
    let mut last_time = crate::interrupts::time_since_boot();
    loop {
        if last_time + std::time::Duration::from_millis(100) < crate::interrupts::time_since_boot() {
            last_time = crate::interrupts::time_since_boot();
            snake::tick(&mut state);
        }
    }
    loop {
        unsafe {
            core::arch::asm!("mov rax, 0x0");
            core::arch::asm!("mov rax, 0x0");
            core::arch::asm!("mov rax, 0x0");
            core::arch::asm!("mov rax, 0x0");
            core::arch::asm!("mov rax, 0x0");
            core::arch::asm!("mov rax, 0x0");
            core::arch::asm!("mov rax, 0x0");
        }
    }
}
