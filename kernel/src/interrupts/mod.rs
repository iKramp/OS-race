mod acpi;
mod gdt;
#[macro_use]
mod handlers;
mod idt;

pub fn init_interrupts(rsdp_addr: Option<u64>) {
    acpi::enable_interrupt_controller(rsdp_addr);
    gdt::init_gdt();
    idt::init_idt();
}
