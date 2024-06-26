mod gdt;
use crate::println;
use core::arch::asm;
#[macro_use]
mod handlers;
mod idt;

pub fn init_interrupts() {
    println!("initializing PIC");
    init_pic();
    println!("initializing GDT");
    gdt::init_gdt();
    println!("initializing IDT");
    idt::init_idt();
    crate::vga_text::set_vga_text_foreground((0, 255, 0));
    println!("interrupts initialized");
    crate::vga_text::reset_vga_color();
}

pub fn init_pic() {
    const PIC1: u16 = 0x20;
    const PIC2: u16 = 0xA0; /* IO base address for slave PIC */
    const PIC1_COMMAND: u16 = PIC1;
    const PIC1_DATA: u16 = PIC1 + 1;
    const PIC2_COMMAND: u16 = PIC2;
    const PIC2_DATA: u16 = PIC2 + 1;

    byte_to_port(PIC1_COMMAND, 0x11);
    byte_to_port(PIC2_COMMAND, 0x11);

    byte_to_port(PIC1_DATA, 0x20);
    byte_to_port(PIC2_DATA, 0x28);

    byte_to_port(PIC1_DATA, 0x04);
    byte_to_port(PIC2_DATA, 0x02);

    byte_to_port(PIC1_DATA, 0x01);
    byte_to_port(PIC2_DATA, 0x01);

    byte_to_port(PIC1_DATA, 0x03); //change to 0x00 to handle keyboard and timer
    byte_to_port(PIC2_DATA, 0x03);
}

fn byte_to_port(port: u16, byte: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") byte);
    }
}
