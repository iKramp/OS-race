use core::arch::asm;

use bootloader_api::info::FrameBuffer;

pub struct VgaBinding {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bytes_per_pixel: usize,
    pub buffer: *mut u8,
}

#[used]
pub static mut VGA_BINDING: VgaBinding = VgaBinding {
    width: 0,
    height: 0,
    stride: 0,
    bytes_per_pixel: 0,
    buffer: core::ptr::null_mut(),
};

pub fn init_vga_driver(binding: &mut FrameBuffer) {
    init_vga_driver_inner(
        binding.info().width,
        binding.info().height,
        binding.info().stride,
        binding.info().bytes_per_pixel,
        binding.buffer_mut().as_mut_ptr(),
    );
}

fn init_vga_driver_inner(width: usize, height: usize, stride: usize, bytes_pp: usize, buffer: *mut u8) {
    unsafe {
        VGA_BINDING.width = width;
        VGA_BINDING.height = height;
        VGA_BINDING.stride = stride;
        VGA_BINDING.bytes_per_pixel = bytes_pp;
        VGA_BINDING.buffer = buffer;
    }

    super::vga_text::init_vga_text(width, height);
}

pub fn clear_screen() {
    unsafe {
        let max_len = VGA_BINDING.stride * VGA_BINDING.height * VGA_BINDING.bytes_per_pixel;
        for i in 0..(max_len >> 3) {
            asm!(
                "mov qword ptr [{vga_ptr}], 0x0000000000000000",
                vga_ptr = in(reg) VGA_BINDING.buffer.add(i * 8),
            )
        }
        let max_offset = VGA_BINDING.buffer.add(max_len);
        *max_offset.offset(-8) = 0;
        *max_offset.offset(-7) = 0;
        *max_offset.offset(-6) = 0;
        *max_offset.offset(-5) = 0;
        *max_offset.offset(-4) = 0;
        *max_offset.offset(-3) = 0;
        *max_offset.offset(-2) = 0;
        *max_offset.offset(-1) = 0;
    }
}

pub fn draw_pixel(x: usize, y: usize, color: (u8, u8, u8)) {
    unsafe {
        let offset = (y * VGA_BINDING.stride + x * VGA_BINDING.bytes_per_pixel) as isize;
        *VGA_BINDING.buffer.offset(offset) = color.0;
        /*asm!(
            "mov qword ptr [{vga_ptr}], {color}",
            vga_ptr = in(reg) VGA_BINDING.buffer.offset(offset),
            color = in(reg) color,
        );*/
    }
}
