mod font;
mod vga_driver;
mod vga_text;

pub use vga_text::{clear_screen, VGA_TEXT};

pub use vga_driver::init_vga_driver;
