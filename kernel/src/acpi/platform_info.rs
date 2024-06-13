use super::madt::MadtEntryType;
use std::mem_utils::PhysAddr;

#[derive(Default)]
pub struct PlatformInfo {
    pub apic: Apic,
    pub processors: std::Vec<Processor>,
}

impl PlatformInfo {
    pub fn new(madt_entries: &std::Vec<super::madt::MadtEntryType>, apic_address: PhysAddr) -> Self {
        let mut info = Self::default();
        info.apic.lapic_address = apic_address;
        for entry in madt_entries {
            match entry {
                MadtEntryType::ProcessorLocalAPIC(data) => info.processors.push(Processor {
                    processor_id: data.acpi_processor_uid,
                    apic_id: data.apic_id,
                    flags: data.flags,
                }),
                MadtEntryType::IoApic(data) => info.apic.io_apics.push(IOApic {
                    id: data.io_apic_id,
                    address: data.io_apic_address,
                    global_system_interrupt_base: data.global_system_interrupt_base,
                }),
                MadtEntryType::LocalApicAddressOverride(data) => info.apic.lapic_address = PhysAddr(data.local_apic_address),
                MadtEntryType::NMISource(data) => {
                    //info.apic.nmi_source.push()
                }
                _ => {}
            }
        }
        //read entries
        info
    }
}

#[derive(Default)]
pub struct Apic {
    pub lapic_address: PhysAddr,
    pub io_apics: std::Vec<IOApic>,
    pub local_apic_nmi_lines: std::Vec<()>,
    pub interrupt_source_overrides: std::Vec<()>,
    pub nmi_source: std::Vec<()>,
}

pub struct Processor {
    pub processor_id: u8,
    pub apic_id: u8,
    pub flags: super::madt::ProcessorLocalApicFlags,
}

pub struct IOApic {
    id: u8,
    address: u32,
    global_system_interrupt_base: u32,
}

pub struct NMISource {
    nmi_source: u8,
}
