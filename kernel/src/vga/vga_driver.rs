//FOR NOW ONLY FULL BYTE COLORS ARE SUPPORTED (32)
#![allow(clippy::identity_op)]

use core::arch::asm;
use crate::println;

#[derive(Debug)]
pub struct FrameBuffer {
    pub buffer: *mut u8,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bits_per_pixel: usize,
    pub blue_offset: usize,
    pub green_offset: usize,
    pub red_offset: usize,
    pub blue_size: usize,
    pub green_size: usize,
    pub red_size: usize,
}

#[used]
pub static mut VGA_BINDING: FrameBuffer = FrameBuffer {
    width: 0,
    height: 0,
    stride: 0,
    bits_per_pixel: 0,
    buffer: core::ptr::null_mut(),
    blue_offset: 0,
    green_offset: 0,
    red_offset: 0,
    blue_size: 0,
    green_size: 0,
    red_size: 0,
};

pub fn init_vga_driver(binding: &FrameBuffer) {
    unsafe {
        VGA_BINDING.width = binding.width;
        VGA_BINDING.height = binding.height;
        VGA_BINDING.stride = binding.stride;
        VGA_BINDING.bits_per_pixel = binding.bits_per_pixel;
        VGA_BINDING.buffer = binding.buffer;
        VGA_BINDING.blue_offset = binding.blue_offset;
        VGA_BINDING.green_offset = binding.green_offset;
        VGA_BINDING.red_offset = binding.red_offset;
        VGA_BINDING.blue_size = binding.blue_size;
        VGA_BINDING.green_size = binding.green_size;
        VGA_BINDING.red_size = binding.red_size;
    }
    super::vga_text::init_vga_text(binding.width, binding.height);
}

pub fn clear_screen() {
    unsafe {
        let max_len = VGA_BINDING.stride * VGA_BINDING.height;
        for i in 0..(max_len >> 3) {
            asm!(
                "mov qword ptr [{vga_ptr}], 0x00000000000000000",
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
        let offset = (y * VGA_BINDING.stride + x * (VGA_BINDING.bits_per_pixel >> 3)) as isize;
        for pixel_byte in (VGA_BINDING.blue_offset >> 3)..((VGA_BINDING.blue_offset + VGA_BINDING.blue_size) >> 3) {
            *VGA_BINDING.buffer.offset(offset + pixel_byte as isize) = color.0;
        }
        for pixel_byte in (VGA_BINDING.green_offset >> 3)..((VGA_BINDING.green_offset + VGA_BINDING.green_size) >> 3) {
            *VGA_BINDING.buffer.offset(offset + pixel_byte as isize) = color.1;
        }
        for pixel_byte in (VGA_BINDING.red_offset >> 3)..((VGA_BINDING.red_offset + VGA_BINDING.red_size) >> 3) {
            *VGA_BINDING.buffer.offset(offset + pixel_byte as isize) = color.2;
        }
    }
}

pub fn draw_rectangle(x: usize, y: usize, width: usize, height: usize, color: (u8, u8, u8)) {
    for i in 0..width {
        for j in 0..height {
            draw_pixel(x + i, y + j, color);
        }
    }
}
