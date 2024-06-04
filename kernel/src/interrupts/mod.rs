mod acpi;
mod gdt;
#[macro_use]
mod handlers;
mod idt;

pub fn init_interrupts() {
    acpi::init_PIC();
    gdt::init_gdt();
    idt::init_idt();
}

pub fn init_apic(rsdp_addr: Option<u64>) {
    acpi::enable_apic(rsdp_addr);
}
