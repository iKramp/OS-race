mod gdt;
use std::{println, printlnc};
pub use gdt::{create_new_gdt, load_gdt, STATIC_GDT_PTR, KERNEL_STACK_SIZE};
#[macro_use]
pub mod handlers;
pub mod idt;
mod macros;
pub use macros::ProcData;
use crate::utils::byte_to_port;

pub fn init_interrupts() {
    println!("initializing PIC");
    init_pic();
    println!("initializing GDT");
    gdt::init_boot_gdt(); //add a separate TSS for each core
    println!("initializing IDT");
    idt::init_idt();
    unsafe { core::arch::asm!("hlt") };
    println!("Some println");
    printlnc!((0, 255, 0), "interrupts initialized");

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
}

pub static mut LEGACY_PIC_TIMER_TICKS: u64 = 0;
pub static mut TIMER_TICKS: u64 = 0;
pub const PIC_TIMER_FREQUENCY: u32 = 1000;
pub const PIC_TIMER_ORIGINAL_FREQ: u32 = 1193180;

pub fn time_since_boot() -> std::time::Duration {
    std::time::Duration::from_nanos(unsafe { TIMER_TICKS } * 1_000_000_000 / PIC_TIMER_FREQUENCY as u64)
}

fn init_timer() {
    const DIVISOR: u16 = (PIC_TIMER_ORIGINAL_FREQ / (PIC_TIMER_FREQUENCY)) as u16;

    #[allow(clippy::unusual_byte_groupings)]
    byte_to_port(0x43, 0b00_11_011_0);
    byte_to_port(0x40, (DIVISOR & 0xFF) as u8);
    byte_to_port(0x40, ((DIVISOR >> 8) & 0xFF) as u8);
}

pub fn disable_timer() {
    //#[allow(clippy::unusual_byte_groupings)]
    //byte_to_port(0x43, 0b00_11_000_0);
    //byte_to_port(0x40, (1000 & 0xFF) as u8);
    //byte_to_port(0x40, ((1000 >> 8) & 0xFF) as u8);
    //another interrupt will be triggered after this, then it stops
}
