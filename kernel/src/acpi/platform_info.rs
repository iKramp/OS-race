use super::madt::MadtEntryType;
use crate::println;
use std::mem_utils::PhysAddr;

#[derive(Default, Debug)]
pub struct PlatformInfo {
    pub apic: Apic,
    pub boot_processor: Option<Processor>,
    pub application_processors: std::Vec<Processor>,
}

impl PlatformInfo {
    pub fn new(madt_entries: &std::Vec<super::madt::MadtEntryType>, apic_address: PhysAddr) -> Self {
        let mut info = Self::default();
        info.apic.lapic_address = apic_address;
        for entry in madt_entries {
            match entry {
                MadtEntryType::ProcessorLocalAPIC(data) => {
                    println!("local apic");
                    if info.boot_processor.is_some() {
                        info.application_processors.push(Processor {
                            processor_id: data.acpi_processor_uid,
                            apic_id: data.apic_id,
                            flags: data.flags,
                        })
                    } else {
                        info.boot_processor = Some(Processor {
                            processor_id: data.acpi_processor_uid,
                            apic_id: data.apic_id,
                            flags: data.flags,
                        })
                    }
                }
                MadtEntryType::IoApic(data) => {
                    println!("IO Apic");
                    info.apic.io_apics.push(IOApic {
                        id: data.io_apic_id,
                        address: data.io_apic_address,
                        global_system_interrupt_base: data.global_system_interrupt_base,
                    });
                }
                MadtEntryType::LocalApicAddressOverride(data) => {
                    println!("local apic address override");
                    info.apic.lapic_address = PhysAddr(data.local_apic_address);
                }
                MadtEntryType::NMISource(_data) => {
                    todo!("nmi sources are not yet implemented");
                    //info.apic.nmi_source.push() //osdev wiki and uefi.org have conflicting
                    //definitions
                }
                MadtEntryType::LocalApicNMI(data) => {
                    println!("local apic nmi");
                    let target = if data.acpi_processor_uid == 0xff {
                        NMILineTarget::All
                    } else {
                        NMILineTarget::Id(data.acpi_processor_uid)
                    };
                    info.apic.local_apic_nmi_lines.push(NMILine {
                        target,
                        flags: data.flags,
                        lint: data.local_apic_lint,
                    });
                }
                MadtEntryType::InterruptSourceOverride(data) => {
                    println!("interrupt source override");
                    if data.bus != 0 {
                        panic!("bus isn't 0 which is not supported");
                    }
                    info.apic.interrupt_source_overrides.push(InterruptSourceOverride {
                        source: data.source,
                        global_system_interrupt: data.global_system_interrupt,
                        flags: data.flags,
                    });
                }
                _ => {}
            }
        }
        println!("apic address is {:#x?}", info.apic.lapic_address);
        info
    }
}

#[derive(Default, Debug)]
pub struct Apic {
    pub lapic_address: PhysAddr,
    pub io_apics: std::Vec<IOApic>,
    pub local_apic_nmi_lines: std::Vec<NMILine>,
    pub interrupt_source_overrides: std::Vec<InterruptSourceOverride>,
    pub nmi_source: std::Vec<()>,
}

#[derive(Debug)]
pub struct Processor {
    pub processor_id: u8,
    pub apic_id: u8,
    pub flags: super::madt::ProcessorLocalApicFlags,
}

#[derive(Debug)]
pub struct IOApic {
    id: u8,
    address: u32,
    global_system_interrupt_base: u32,
}

#[derive(Debug)]
pub struct InterruptSourceOverride {
    source: u8,
    global_system_interrupt: u32,
    flags: super::madt::MpsIntiFlags,
}

pub struct LocalAPICNMI {
    processor_id: u8,
    flags: super::madt::MpsIntiFlags,
    local_apic_lint: u8,
}

#[derive(Debug)]
pub enum NMILineTarget {
    All,
    Id(u8),
}
#[derive(Debug)]
pub struct NMILine {
    target: NMILineTarget,
    flags: super::madt::MpsIntiFlags,
    lint: u8,
}
