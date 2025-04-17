use std::{
    PageAllocator,
    mem_utils::{PhysAddr, get_at_virtual_addr},
};

use super::platform_info::PlatformInfo;
use crate::memory::{paging::LiminePat, physical_allocator};

pub fn init_ioapic(platform_info: &PlatformInfo) {
    unsafe {
        for io_apic_info in &platform_info.apic.io_apics {
            physical_allocator::mark_addr(PhysAddr(io_apic_info.address.into()), true);
            let io_apic_address = crate::memory::PAGE_TREE_ALLOCATOR.allocate(Some(PhysAddr(io_apic_info.address.into())));
            let apic_registers_page_entry = crate::memory::PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(io_apic_address)
                .unwrap();
            apic_registers_page_entry.set_pat(LiminePat::UC);
            core::arch::asm!(
                "mov rax, cr3",
                "mov cr3, rax",
                out("rax") _
            ); //clear the TLB
            let io_apic = get_at_virtual_addr::<IoApicRegisters>(io_apic_address);
            let (_, entries) = io_apic.get_version_and_entries();
            //println!("got io apic: {:#x?}", io_apic);
            //println!("io apic version: {:#x?}", (version, entries));
            //println!("io apic id: {:#x?}", io_apic.get_id());

            for gsi in 0..entries - 1 {
                let mut trigger_mode = 0;
                let mut polarity = 0;
                let mut vector = gsi;
                for interrupt_override in &platform_info.apic.interrupt_source_overrides {
                    if interrupt_override.global_system_interrupt as u8 != gsi {
                        continue;
                    }
                    //println!("{:#x?}", interrupt_override);
                    if let crate::acpi::madt::IntSoOverTriggerMode::LevelTriggered = interrupt_override.flags.trigger_mode() {
                        trigger_mode = 1;
                    }
                    if let crate::acpi::madt::IntSoOverPolarity::ActiveLow = interrupt_override.flags.polarity() {
                        polarity = 1;
                    }
                    vector = interrupt_override.source;
                }

                let table_entry = RedTblEntry::new(
                    platform_info.boot_processor.apic_id,
                    0,
                    trigger_mode,
                    0,
                    polarity,
                    0,
                    0,
                    vector + 32,
                );
                //println!("setting entry {:b}", table_entry.0);
                io_apic.set_redir_table(gsi, table_entry)
            }
        }
    }
}

#[derive(Debug)]
#[repr(C, packed)]
struct IoApicRegisters {
    pub index: u8,
    padding_0: u8,
    padding_1: u16,
    padding_2: u32,
    padding_3: u64,
    pub data: u32,
    padding_4: u32,
    padding_5: u64,
    irq_pin_assertion: u32,
    padding_6: u32,
    padding_7: u64,
    padding_8: u128,
    eoi: u32,
}

impl IoApicRegisters {
    pub fn get_version_and_entries(&mut self) -> (u8, u8) {
        self.index = 1;
        let ver_entries = self.data;
        let version = ver_entries & 0xFF;
        let entries = (ver_entries >> 16) & 0xFF;
        (version as u8, entries as u8)
    }

    pub fn get_id(&mut self) -> u8 {
        self.index = 0;
        (self.data >> 24) as u8 & 0xF
    }

    pub fn get_redir_table(&mut self, index: u8) -> RedTblEntry {
        self.index = 0x10 + index * 2;
        let low = self.data as u64;
        self.index = 0x11 + index * 2;
        let high = self.data as u64;
        RedTblEntry(low + (high << 32))
    }

    pub fn set_redir_table(&mut self, index: u8, entry: RedTblEntry) {
        let low: u32 = (entry.0 & 0xFFFFFFFF) as u32;
        let high: u32 = ((entry.0 >> 32) & 0xFFFFFFFF) as u32;
        self.index = 0x10 + index * 2;
        self.data = low;
        self.index = 0x11 + index * 2;
        self.data = high;
    }
}

#[derive(Debug)]
struct RedTblEntry(pub u64);

impl RedTblEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        destination: u8,
        mask: u8,
        trigger_mode: u8,
        remote_irr: u8,
        interrupt_pin_polarity: u8,
        destination_mode: u8,
        delivery_mode: u8,
        vector: u8,
    ) -> Self {
        let mut num = vector as u64;
        num |= (delivery_mode as u64 & 0b111) << 8;
        num |= (destination_mode as u64 & 0b1) << 11;
        num |= (interrupt_pin_polarity as u64 & 0b111) << 13;
        num |= (remote_irr as u64 & 0b111) << 14;
        num |= (trigger_mode as u64 & 0b111) << 15;
        num |= (mask as u64 & 0b111) << 16;
        num |= (destination as u64) << 48;

        Self(num)
    }
}
