mod fadt;
mod madt;
mod rsdp;
mod rsdt;
mod sdt;

pub fn init_acpi(rsdp_address: std::option::Option<u64>) {
    let Some(rsdp) = rsdp::get_rsdp_table(rsdp_address) else {
        return;
    };
    let rsdt = rsdt::get_rsdt(&rsdp);
    assert!(rsdt.validate());
    rsdt.print_signature();
    crate::println!("{:#?}", rsdt);
    rsdt.print_tables();

    //these are apparently important according to the osdev wiki
    //also i'll need to do something differently because SOME tables can appear multiple times
    let _fadt = rsdt
        .get_table(*b"FACP")
        .map(|addr| unsafe { std::mem_utils::get_at_physical_addr::<fadt::Fadt>(addr) })
        .expect("fadt should be present");
    let madt = rsdt
        .get_table(*b"APIC")
        .map(|addr| unsafe { std::mem_utils::get_at_physical_addr::<madt::Madt>(addr) })
        .expect("madt should be present");
    let _ssdt = rsdt.get_table(*b"SSDT"); //we'll just use dsdt

    let _entries = madt.get_madt_entries();
    //override madt apic address if it exists in entries
}
