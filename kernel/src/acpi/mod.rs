mod fadt;
mod madt;
mod platform_info;
mod rsdp;
mod rsdt;
mod sdt;

pub fn init_acpi(rsdp_address: std::option::Option<u64>) {
    let Some(rsdp) = rsdp::get_rsdp_table(rsdp_address) else {
        return;
    };
    let rsdt = rsdt::get_rsdt(&rsdp);
    assert!(rsdt.validate());

    let mut fadt = None;
    let mut madt = None;

    for table in &rsdt.get_tables() {
        unsafe {
            let header = std::mem_utils::get_at_physical_addr::<sdt::AcpiSdtHeader>(*table);
            match &header.signature {
                b"FACP" => fadt = Some(std::mem_utils::get_at_physical_addr::<fadt::Fadt>(*table)),
                b"APIC" => madt = Some(std::mem_utils::get_at_physical_addr::<madt::Madt>(*table)),
                _ => {}
            }
        }
    }

    let _fadt = fadt.expect("fadt should be present");
    let madt = madt.expect("madt should be present");

    let entries = madt.get_madt_entries();
    let _platform_info = platform_info::PlatformInfo::new(&entries, std::mem_utils::PhysAddr(madt.local_apic_address as u64));
    //override madt apic address if it exists in entries

    //after loading dsdt

    for table in &rsdt.get_tables() {
        unsafe {
            let header = std::mem_utils::get_at_physical_addr::<sdt::AcpiSdtHeader>(*table);
            if &header.signature == b"SSDT" {
                //parse secondary tables
            }
        }
    }
}
