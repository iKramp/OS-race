mod aml;
mod apic;
mod fadt;
mod ioapic;
mod lapic_timer;
mod madt;
mod platform_info;
mod rsdp;
mod rsdt;
mod sdt;
mod smp;
mod timer;

use core::mem::MaybeUninit;
use std::{Vec, mem_utils::PhysAddr};

pub use apic::LAPIC_REGISTERS;
pub use lapic_timer::time_since_boot;
use platform_info::PlatformInfo;
pub use smp::cpu_locals;

use crate::{
    interrupts::{APIC_TIMER_INIT, APIC_TIMER_TICKS},
    limine::LIMINE_BOOTLOADER_REQUESTS,
    memory::PAGE_TREE_ALLOCATOR,
    println, printlnc,
};

static mut PLATFORM_INFO: Option<PlatformInfo> = None;
pub fn get_platform_info() -> &'static PlatformInfo {
    unsafe {
        let Some(platform_info) = &PLATFORM_INFO else {
            panic!("platform info not initialized");
        };
        platform_info
    }
}

pub fn init_acpi() {
    let rsdp = rsdp::get_rsdp_table(unsafe { (*LIMINE_BOOTLOADER_REQUESTS.rsdp_request.info).rsdp as u64 })
        .expect("This os doesn not support PCs without ACPI");
    let rsdt = rsdt::get_rsdt(&rsdp);
    assert!(rsdt.validate());
    println!("rsdt is valid");

    let mut fadt = None;
    let mut madt = None;
    let mut hpet = None;

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
                b"HPET" => hpet = Some(std::mem_utils::get_at_physical_addr::<timer::hpet::HpetTable>(*table)),
                _ => {} //any other tables except SSDT, those are parsed after DSDT
                        //qemu only reports FACP, APIC, HPET and WAET
            }
        }
    }
    println!("tables parsed");

    let fadt = fadt.expect("fadt should be present");
    let madt = madt.expect("madt should be present");
    let hpet = hpet.expect("hpet should be present");

    unsafe { timer::HPET_ACPI_TABLE = MaybeUninit::new(hpet) };
    timer::init();

    let entries = madt.get_madt_entries();
    let platform_info = platform_info::PlatformInfo::new(&entries, std::mem_utils::PhysAddr(madt.local_apic_address as u64));
    //override madt apic address if it exists in entries
    println!("initing APIC");
    let platform_info = unsafe {
        PLATFORM_INFO = Some(platform_info);
        let Some(platform_info) = &PLATFORM_INFO else {
            panic!("a");
        };
        platform_info
    };
    unsafe {
        APIC_TIMER_INIT = true;
        let slots = platform_info.application_processors.len() + 1;
        #[allow(clippy::slow_vector_initialization)] //it's non const ffs
        let mut vec = Vec::with_capacity(slots);
        vec.resize(slots, 0);
        APIC_TIMER_TICKS = MaybeUninit::new(vec.into_boxed_slice());
    };
    cpu_locals::init(platform_info);

    apic::enable_apic(platform_info, platform_info.boot_processor.processor_id);
    ioapic::init_ioapic(platform_info);

    smp::wake_cpus(platform_info);
    printlnc!((0, 255, 0), "ACPI initialized and APs started");
    unsafe { PAGE_TREE_ALLOCATOR.unmap_lower_half() };

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

    let _dsdt_addr = std::mem_utils::translate_phys_virt_addr(PhysAddr(fadt.dsdt as u64));
    //let _aml_code = aml::AmlCode::new(dsdt_addr.0 as *const u8);
}

pub fn init_acpi_ap(processor_id: u8) {
    unsafe {
        let Some(platform_info) = &PLATFORM_INFO else {
            panic!("should be impossible, acpi tables are not loaded but APs were initialized");
        };
        apic::enable_apic(platform_info, processor_id);
    }
}
