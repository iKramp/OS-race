#![no_std]
#![no_main]

use core::{borrow::BorrowMut, panic::PanicInfo};

use bootloader_api::{config::Mapping, entry_point, BootloaderConfig};

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

static HELLO: &[u8] = b"Hello World!";

#[no_mangle]
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    let binding = boot_info.framebuffer.as_mut().unwrap();
    assert_eq!(
        binding.info().pixel_format,
        bootloader_api::info::PixelFormat::Bgr
    );
    let width = binding.info().width;
    let height = binding.info().height;
    let stride = binding.info().stride;
    let bytes_per_pixel = binding.info().bytes_per_pixel;
    let framebuffer = binding.buffer_mut();

    for y in 0..height {
        for x in 0..width {
            for i in 0..bytes_per_pixel {
                let offset = (y * stride + x) * bytes_per_pixel + i;
                unsafe {
                    *framebuffer.as_mut_ptr().add(offset) = 0;
                }
            }
        }
    }

    loop {}
}
