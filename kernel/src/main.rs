#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(stmt_expr_attributes)]

use std::panic::PanicInfo;

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
use vga::vga_text;

use crate::limine::FramebufferMode;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_text::set_vga_text_foreground((0, 0, 255));
    println!("{}", info);
    std::panic::print_stack_trace();
    loop {}
}

pub struct BootInfo {}

#[no_mangle]
extern "C" fn _start() -> ! {
    let framebuffer_info = unsafe { &*LIMINE_BOOTLOADER_REQUESTS.frame_buffer_request.info };

    if framebuffer_info.framebuffer_count == 0 {
        panic!("No framebuffers found");
    }

    let framebuffer_slice = unsafe { core::slice::from_raw_parts(framebuffer_info.framebuffers as *const *const limine::FramebufferInfo, framebuffer_info.framebuffer_count as usize) };
    let main_framebuffer = unsafe { &*framebuffer_slice[0] };

    let _modes = if framebuffer_info.revision >= 1 {
        unsafe { core::slice::from_raw_parts(main_framebuffer.modes as *const *const limine::FramebufferMode, main_framebuffer.mode_count as usize) }
    } else {
        #[allow(clippy::invalid_null_ptr_usage)]
        unsafe { core::slice::from_raw_parts(core::ptr::null::<*const limine::FramebufferMode>(), 0)}
    };

    let my_vga_binding = vga::vga_driver::FrameBuffer {
        width:  main_framebuffer.width as usize,
        height: main_framebuffer.height as usize,
        stride: main_framebuffer.pitch as usize,
        bits_per_pixel: main_framebuffer.bpp as usize,
        buffer: main_framebuffer.address as *mut u8,
        blue_offset: main_framebuffer.blue_mask_shift as usize,
        green_offset: main_framebuffer.green_mask_shift as usize,
        red_offset: main_framebuffer.red_mask_shift as usize,
        blue_size: main_framebuffer.blue_mask_size as usize,
        green_size: main_framebuffer.green_mask_size as usize,
        red_size: main_framebuffer.red_mask_size as usize,
    };

    vga::init_vga_driver(&my_vga_binding);
    vga::clear_screen();

    println!("starting RustOs...");

    let boot_info = unsafe { &*LIMINE_BOOTLOADER_REQUESTS.bootloader_info_request.info }; 
    let name = utils::ptr_to_str(boot_info.name);
    println!("Booted with bootloader: {:?}", name);
    let version = utils::ptr_to_str(boot_info.version);
    println!("Version: {}", version);

    interrupts::init_interrupts();

    memory::init_memory();

    acpi::init_acpi();

    //vga_text::hello_message();

    //let last_time = crate::interrupts::time_since_boot();
    /*loop {
        if last_time + std::time::Duration::from_millis(500) < crate::interrupts::time_since_boot() {
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
    }*/
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
