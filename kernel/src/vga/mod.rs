mod font;
mod vga_driver;
pub mod vga_text;

pub use vga_text::clear_screen;

pub use vga_driver::init_vga_driver;
