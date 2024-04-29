#![no_std]
#![no_main]

use bootloader_api::entry_point;
use core::panic::PanicInfo;

mod interrupts;
#[allow(unused_imports)]
mod tests;
mod vga;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::vga_text::set_vga_text_foreground((0, 0, 255));
    println!("{}", info);
    loop {}
}

entry_point!(kernel_main);

#[no_mangle]
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    let binding = boot_info.framebuffer.as_mut().unwrap();
    assert_eq!(
        binding.info().pixel_format,
        bootloader_api::info::PixelFormat::Bgr
    );
    vga::init_vga_driver(
        binding.info().width,
        binding.info().height,
        binding.info().stride,
        binding.info().bytes_per_pixel,
        binding.buffer_mut().as_mut_ptr(),
    );

    vga::clear_screen();

    interrupts::setup_idt();

    print!("Hello via ");

    vga::vga_text::set_vga_text_foreground((30, 105, 210));

    println!("RustOS");

    #[cfg(feature = "run_tests")]
    {
        if true {
            println!("Hello world");
            use crate::tests::test_runner;
            test_runner();
        }
    }

    println!("This message is created after tests");

    #[allow(clippy::empty_loop)]
    loop {}
}

/*pub unsafe fn exit_qemu(ok: bool) {
    if ok {
        asm!(
            "mov eax, 0x10",
            "out 0xf4, eax",
            out("eax") _
        );
    } else {
        asm!(
            "mov eax, 0x11",
            "out 0xf4, eax",
            out("eax") _
        );
    }
}*/
