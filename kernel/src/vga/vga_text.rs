use super::font::*;
use super::vga_driver::VGA_BINDING;
use core::arch::asm;

pub struct VgaText {
    pub foreground: (u8, u8, u8),
    pub background: (u8, u8, u8),
    height_lines: usize,
    width_chars: usize,
    line: usize,
    char: usize,
    offset: usize,
}

impl VgaText {
    pub fn write_text(&mut self, text: &str) {
        for char in text.as_bytes() {
            if char == &b'\n' {
                self.do_newline();
            } else {
                unsafe {
                    self.write_character(char);
                }
            }
        }
    }

    fn do_newline(&mut self) {
        self.char = 0;
        self.line += 1;
        self.offset =
            unsafe { VGA_BINDING.stride * self.line * CHAR_HEIGHT * VGA_BINDING.bytes_per_pixel };
    }

    pub unsafe fn write_character(&mut self, character: &u8) {
        let character = &DEFAULT_FONT[*character as usize * 8..(*character as usize + 1) * 8];
        let mut curr_off = self.offset;
        for char_line in character {
            for i in 0..8 {
                let bit = char_line & (128 >> i) != 0;
                let color = match bit {
                    true => self.foreground,
                    false => self.background,
                };
                *VGA_BINDING
                    .buffer
                    .add(curr_off + i * VGA_BINDING.bytes_per_pixel) = color.0;
                *VGA_BINDING
                    .buffer
                    .add(curr_off + i * VGA_BINDING.bytes_per_pixel + 1) = color.1;
                *VGA_BINDING
                    .buffer
                    .add(curr_off + i * VGA_BINDING.bytes_per_pixel + 2) = color.2;
            }
            curr_off += unsafe { VGA_BINDING.stride * VGA_BINDING.bytes_per_pixel };
        }

        self.char += 1;
        self.offset += CHAR_WIDTH * unsafe { VGA_BINDING.bytes_per_pixel };
        if self.char > self.width_chars {
            self.do_newline();
        }
    }
}

pub static mut VGA_TEXT: VgaText = VgaText {
    background: (0, 0, 0),
    foreground: (255, 255, 255),
    height_lines: 0,
    width_chars: 0,
    line: 0,
    char: 0,
    offset: 0,
};

pub fn init_vga_text(width: usize, height: usize) {
    unsafe {
        VGA_TEXT.height_lines = height / CHAR_HEIGHT;
        VGA_TEXT.width_chars = width / CHAR_WIDTH;
    }
}
