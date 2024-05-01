mod gdt;
#[macro_use]
mod handlers;

mod idt;

pub fn init_interrupts() {
    gdt::init_gdt();
    idt::init_idt();
}
