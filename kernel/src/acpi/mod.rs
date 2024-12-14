mod apic;
mod aml;
mod smp;
mod fadt;
mod ioapic;
mod madt;
mod platform_info;
mod rsdp;
mod rsdt;
mod sdt;

use std::{Box, Vec};

pub use apic::{LapicRegisters, LAPIC_REGISTERS};
use platform_info::PlatformInfo;

use crate::{limine::LIMINE_BOOTLOADER_REQUESTS, memory::physical_allocator::BUDDY_ALLOCATOR, println, printlnc};

static mut PLATFORM_INFO: Option<PlatformInfo> = None;

pub fn init_acpi() {
    let rsdp = rsdp::get_rsdp_table(unsafe { (*LIMINE_BOOTLOADER_REQUESTS.rsdp_request.info).rsdp as u64 }).expect("This os doesn not support PCs without ACPI");
    let rsdt = rsdt::get_rsdt(&rsdp);
    assert!(rsdt.validate());
    println!("rsdt is valid");

    let mut fadt = None;
    let mut madt = None;

    let tables = rsdt.get_tables();
    for table in &tables {
        //print header signatures
        unsafe {
            let header = std::mem_utils::get_at_physical_addr::<sdt::AcpiSdtHeader>(*table);
            let mut signature_clone = Vec::new();
            for i in 0..4 {
                signature_clone.push(header.signature[i]);
            }
            let signature = std::string::String::from_utf8(signature_clone).unwrap();
            println!("{}", signature);
        }
    }
    for table in &tables {
        unsafe {
            let header = std::mem_utils::get_at_physical_addr::<sdt::AcpiSdtHeader>(*table);
            match &header.signature {
                b"FACP" => fadt = Some(std::mem_utils::get_at_physical_addr::<fadt::Fadt>(*table)),
                b"APIC" => madt = Some(std::mem_utils::get_at_physical_addr::<madt::Madt>(*table)),
                _ => {} //any other tables except SSDT, those are parsed after DSDT
                        //qemu only reports FACP, APIC, HPET and WAET
            }
        }
    }
    println!("tables parsed");

    let _fadt = fadt.expect("fadt should be present");
    let madt = madt.expect("madt should be present");

    let entries = madt.get_madt_entries();
    let platform_info = platform_info::PlatformInfo::new(&entries, std::mem_utils::PhysAddr(madt.local_apic_address as u64));
    //override madt apic address if it exists in entries
    println!("initing APIC");
    let platform_info = unsafe {
        BUDDY_ALLOCATOR.mark_addr(platform_info.apic.lapic_address, true);
        PLATFORM_INFO = Some(platform_info);
        let Some(platform_info) = &PLATFORM_INFO else {
            panic!("a");
        };
        platform_info
    };
    apic::enable_apic(&platform_info, platform_info.boot_processor.processor_id);
    ioapic::init_ioapic(&platform_info);
    smp::wake_cpus(&platform_info);
    printlnc!((0, 255, 0), "ACPI initialized and APs started");

    //after loading dsdt
    /*
        for table in &rsdt.get_tables() {
            unsafe {
                let header = std::mem_utils::get_at_physical_addr::<sdt::AcpiSdtHeader>(*table);
                if &header.signature == b"SSDT" {
                    //parse secondary tables
                    //actually don't this shit is difficult af
                }
            }
        }
    */
}

pub fn init_acpi_ap(processor_id: u8) {
    unsafe {
        let Some(platform_info) = &PLATFORM_INFO else {
            panic!("should be impossible, acpi tables are not loaded but APs were initialized");
        };
        apic::enable_apic(platform_info, processor_id);
    }
}
