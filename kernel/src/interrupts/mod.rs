mod gdt;
use core::mem::MaybeUninit;
pub use gdt::{STATIC_GDT_PTR, create_new_gdt, load_gdt};
use std::{boxed::Box, println, printlnc};
#[macro_use]
pub mod handlers;
pub mod idt;
mod macros;
use crate::utils::byte_to_port;
pub use macros::InterruptProcessorState;

const PIC1: u16 = 0x20;
const PIC2: u16 = 0xA0; /* IO base address for slave PIC */
const PIC1_COMMAND: u16 = PIC1;
const PIC1_DATA: u16 = PIC1 + 1;
const PIC2_COMMAND: u16 = PIC2;
const PIC2_DATA: u16 = PIC2 + 1;

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
    byte_to_port(PIC1_COMMAND, 0x11);
    byte_to_port(PIC2_COMMAND, 0x11);

    byte_to_port(PIC1_DATA, 0x20);
    byte_to_port(PIC2_DATA, 0x28);

    byte_to_port(PIC1_DATA, 0x04);
    byte_to_port(PIC2_DATA, 0x02);

    byte_to_port(PIC1_DATA, 0x01);
    byte_to_port(PIC2_DATA, 0x01);

    init_timer();

    byte_to_port(PIC1_DATA, 0xFE); //only allow timer
    byte_to_port(PIC2_DATA, 0xFE);
}

pub fn disable_pic_keep_timer() {
    byte_to_port(PIC1_DATA, 0xFE); //mask interrupts, keep timer
    byte_to_port(PIC2_DATA, 0xFE);

    byte_to_port(PIC1_DATA - 1, 0x20); //trigger EOI
    byte_to_port(PIC2_DATA - 1, 0x20);
}

pub fn disable_pic_completely() {
    byte_to_port(PIC1_DATA, 0xFF); //mask interrupts
    byte_to_port(PIC2_DATA, 0xFF);

    byte_to_port(PIC1_DATA - 1, 0x20); //trigger EOI
    byte_to_port(PIC2_DATA - 1, 0x20);

    disable_timer();

    disconnect_imcr();
}

fn disconnect_imcr() {
    const IMCR: u16 = 0x22;

    byte_to_port(IMCR, 0x70);
    byte_to_port(IMCR + 1, 0x01);
}

pub static mut LEGACY_PIC_TIMER_TICKS: u64 = 0;
pub static mut APIC_TIMER_TICKS: MaybeUninit<Box<[u64]>> = MaybeUninit::uninit();
pub static mut APIC_TIMER_INIT: bool = false;
pub const TIMER_DESIRED_FREQUENCY: u32 = 1; //don't need much lmao
pub const PIC_TIMER_ORIGINAL_FREQ: u32 = 1193180;
pub const PIC_DIVISOR: u16 = 119 * 3;
pub const PIC_ACTUAL_FREQ: u32 = PIC_TIMER_ORIGINAL_FREQ / PIC_DIVISOR as u32;

fn init_timer() {
    #[allow(clippy::unusual_byte_groupings)]
    byte_to_port(0x43, 0b00_11_011_0);
    byte_to_port(0x40, (PIC_DIVISOR & 0xFF) as u8);
    byte_to_port(0x40, ((PIC_DIVISOR >> 8) & 0xFF) as u8);
}

fn disable_timer() {
    #[allow(clippy::unusual_byte_groupings)]
    byte_to_port(0x43, 0b00_11_000_0);
}
