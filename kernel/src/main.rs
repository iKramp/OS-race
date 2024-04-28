#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootloaderConfig};
use core::panic::PanicInfo;

mod vga;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = 100 * 1024;
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

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

    unsafe {
        vga::VGA_TEXT.write_text("TEST\nthis is a sentance in a new line, attempting non-ascii: ");
        vga::VGA_TEXT.write_text("π");
    }

    #[allow(clippy::empty_loop)]
    loop {}
}
