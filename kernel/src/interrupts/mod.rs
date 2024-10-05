mod gdt;
use crate::println;
#[macro_use]
pub mod handlers;
pub mod idt;
use crate::utils::byte_to_port;

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

    init_timer();

    byte_to_port(PIC1_DATA, 0x00); //change to 0x00 to handle keyboard
    byte_to_port(PIC2_DATA, 0x00);

    std::panic::print_stack_trace();
}

pub static mut LEGACY_PIC_TIMER_TICKS: u64 = 0;
pub static mut TIMER_TICKS: u64 = 0;
pub const PIC_TIMER_FREQUENCY: u32 = 59659;
pub const PIC_TIMER_ORIGINAL_FREQ: u32 = 1193180;

pub fn time_since_boot() -> std::time::Duration {
    std::time::Duration::from_nanos(unsafe { TIMER_TICKS } * 1_000_000_000 / PIC_TIMER_FREQUENCY as u64)
}

fn init_timer() {
    const DIVISOR: u16 = (PIC_TIMER_ORIGINAL_FREQ / (PIC_TIMER_FREQUENCY)) as u16;

    byte_to_port(0x43, 0x36);
    byte_to_port(0x40, (DIVISOR & 0xFF) as u8);
    byte_to_port(0x40, ((DIVISOR >> 8) & 0xFF) as u8);
}
