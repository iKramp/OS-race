use core::arch::asm;

use super::font::*;
use super::vga_driver::VGA_BINDING;

struct VgaText {
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
        if self.line >= self.height_lines - 1 {
            self.line -= 1;
            unsafe { self.scroll() };
        }
        self.offset = unsafe { VGA_BINDING.stride * self.line * CHAR_HEIGHT * 2 * VGA_BINDING.bytes_per_pixel };
    }

    pub unsafe fn write_character(&mut self, mut character: &u8) {
        if !(0x20..0x7f).contains(character) {
            character = &0xfe
        }

        let character = &DEFAULT_FONT[*character as usize * 8..(*character as usize + 1) * 8];
        let mut curr_off = self.offset;
        for char_line in character {
            for _ in 0..2 {
                for i in 0..8 {
                    let bit = char_line & (128 >> i) != 0;
                    let color = match bit {
                        true => self.foreground,
                        false => self.background,
                    };
                    *VGA_BINDING.buffer.add(curr_off + i * VGA_BINDING.bytes_per_pixel * 2) = color.0;
                    *VGA_BINDING
                        .buffer
                        .add(curr_off + i * VGA_BINDING.bytes_per_pixel * 2 + 1) = color.1;
                    *VGA_BINDING
                        .buffer
                        .add(curr_off + i * VGA_BINDING.bytes_per_pixel * 2 + 2) = color.2;
                    *VGA_BINDING
                        .buffer
                        .add(curr_off + i * VGA_BINDING.bytes_per_pixel * 2 + 3) = color.0;
                    *VGA_BINDING
                        .buffer
                        .add(curr_off + i * VGA_BINDING.bytes_per_pixel * 2 + 4) = color.1;
                    *VGA_BINDING
                        .buffer
                        .add(curr_off + i * VGA_BINDING.bytes_per_pixel * 2 + 5) = color.2;
                }
                curr_off += unsafe { VGA_BINDING.stride * VGA_BINDING.bytes_per_pixel };
            }
        }

        self.char += 1;
        self.offset += CHAR_WIDTH * unsafe { VGA_BINDING.bytes_per_pixel * 2 };
        if self.char >= self.width_chars {
            self.do_newline();
        }
    }

    unsafe fn scroll(&mut self) {
        let top_ptr = VGA_BINDING.buffer;
        let diff = VGA_BINDING.bytes_per_pixel * VGA_BINDING.stride * CHAR_HEIGHT * 2;
        let limit = top_ptr.add(diff * (self.height_lines - 1) - 1);

        asm!(
            "2:",
            "mov r8, [r10 + r9]",
            "mov qword ptr [r10], r8",
            "add r10, 8",
            "cmp r10, r11",
            "jle 2b",
            in("r9") diff,
            in("r10") top_ptr,
            in("r11") limit,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
            out("r8") _,
        )
    }
}

impl core::fmt::Write for VgaText {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_text(s);
        Ok(())
    }
}

static mut VGA_TEXT: VgaText = VgaText {
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
        VGA_TEXT.height_lines = height / (CHAR_HEIGHT * 2);
        VGA_TEXT.width_chars = width / (CHAR_WIDTH * 2);
    }
}

pub fn set_vga_text_foreground(color: (u8, u8, u8)) {
    unsafe { VGA_TEXT.foreground = color }
}
pub fn set_vga_text_background(color: (u8, u8, u8)) {
    unsafe { VGA_TEXT.background = color }
}
pub fn set_vga_text_colors(fg_color: (u8, u8, u8), bg_color: (u8, u8, u8)) {
    set_vga_text_background(bg_color);
    set_vga_text_foreground(fg_color);
}

pub fn clear_screen() {
    super::vga_driver::clear_screen();
    unsafe {
        VGA_TEXT.line = 0;
        VGA_TEXT.char = 0;
        VGA_TEXT.offset = 0;
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::vga_text::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    unsafe { VGA_TEXT.write_fmt(args).unwrap() };
}
