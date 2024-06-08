#![allow(dead_code)]

use std::{
    mem_utils::{get_at_virtual_addr, VirtAddr},
    vec::Vec,
};

#[repr(C)]
pub struct Madt {
    header: super::sdt::AcpiSdtHeader,
    local_apic_address: u32,
    flags: u32, //unused, indicates if dual setup pic is present but we assume it is and mask it out
                //anyway
}

impl Madt {
    pub fn get_madt_entries(&self) -> Vec<MadtEntryType> {
        let mut entries = Vec::new();
        let mut ptr = VirtAddr((self as *const Madt as u64) + 0x2C);
        while ptr.0 < (self as *const Madt as u64 + self.header.length as u64) {
            unsafe {
                let entry_header = get_at_virtual_addr::<MadtEntryHeader>(ptr);
                match entry_header.entry_type {
                    0 => entries.push(MadtEntryType::ProcessorLocalAPIC(get_at_virtual_addr::<ProcessorLocalApic>(
                        ptr,
                    ))),
                    1 => entries.push(MadtEntryType::IoApic(get_at_virtual_addr::<IoApic>(ptr))),
                    2 => entries.push(MadtEntryType::InterruptSourceOverride(get_at_virtual_addr::<
                        InterruptSourceOverride,
                    >(ptr))),
                    3 => entries.push(MadtEntryType::NMISource(get_at_virtual_addr::<NMISource>(ptr))),
                    4 => entries.push(MadtEntryType::LocalApicNMI(get_at_virtual_addr::<LocalApicNMI>(ptr))),
                    5 => entries.push(MadtEntryType::LocalApicAddressOverride(get_at_virtual_addr::<
                        LocalApicAddressOverride,
                    >(ptr))),
                    9 => entries.push(MadtEntryType::ProcessorLocalX2APIC(
                        get_at_virtual_addr::<ProcessorLocalX2APIC>(ptr),
                    )),
                    _ => entries.push(MadtEntryType::Other),
                }
                ptr.0 += entry_header.length as u64;
            }
        }
        entries
    }
}

pub enum MadtEntryType {
    ProcessorLocalAPIC(&'static ProcessorLocalApic),
    IoApic(&'static IoApic),
    InterruptSourceOverride(&'static InterruptSourceOverride),
    NMISource(&'static NMISource),
    LocalApicNMI(&'static LocalApicNMI),
    LocalApicAddressOverride(&'static LocalApicAddressOverride),
    ProcessorLocalX2APIC(&'static ProcessorLocalX2APIC),

    Other,
}

pub struct MadtEntryHeader {
    entry_type: u8,
    length: u8,
}

pub struct ProcessorLocalApicFlags(u32);

impl ProcessorLocalApicFlags {
    pub fn enabled(&self) -> bool {
        self.0 & 1 == 1
    }
    pub fn online_capable(&self) -> bool {
        self.0 & 2 == 2
    }
}

pub struct ProcessorLocalApic {
    header: MadtEntryHeader,
    acpi_processor_uid: u8,
    apic_id: u8,
    flags: ProcessorLocalApicFlags,
}

pub struct IoApic {
    header: MadtEntryHeader,
    io_apic_id: u8,
    reserved: u8,
    io_apic_address: u32,
    global_system_interrupt_base: u32,
}

pub enum IntSoOverPolarity {
    Conforms,
    ActiveHigh,
    Reserved,
    ActiveLow,
}
pub enum IntSoOverTriggerMode {
    Conforms,
    EdgeTriggered,
    Reserved,
    LevelTriggered,
}
pub struct MpsIntiFlags {
    flags: u16,
}
impl MpsIntiFlags {
    pub fn polarity(&self) -> IntSoOverPolarity {
        match self.flags & 3 {
            0 => IntSoOverPolarity::Conforms,
            1 => IntSoOverPolarity::ActiveHigh,
            2 => IntSoOverPolarity::Reserved,
            3 => IntSoOverPolarity::ActiveLow,
            _ => panic!("impossible"),
        }
    }

    pub fn trigger_mode(&self) -> IntSoOverTriggerMode {
        match self.flags & 0b1100 {
            0b0000 => IntSoOverTriggerMode::Conforms,
            0b0100 => IntSoOverTriggerMode::EdgeTriggered,
            0b1000 => IntSoOverTriggerMode::Reserved,
            0b1100 => IntSoOverTriggerMode::LevelTriggered,
            _ => panic!("impossible"),
        }
    }
}

pub struct InterruptSourceOverride {
    header: MadtEntryHeader,
    bus: u8,
    source: u8,
    global_system_interrupt: u32,
    flags: MpsIntiFlags,
}

pub struct NMISource {
    header: MadtEntryHeader,
    flags: MpsIntiFlags,
    global_system_interrupt: u32,
}

pub struct LocalApicNMI {
    header: MadtEntryHeader,
    acpi_processor_uid: u8,
    flags: MpsIntiFlags,
    local_apic_lint: u8,
}

pub struct LocalApicAddressOverride {
    header: MadtEntryHeader,
    reserved: u16,
    local_apic_address: u64,
}

pub struct ProcessorLocalX2APIC {
    header: MadtEntryHeader,
    reserved: u16,
    x2apic_id: u32,
    flags: ProcessorLocalApicFlags,
    acpi_processor_uid: u32,
}
