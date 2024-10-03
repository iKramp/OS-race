use std::{
    mem_utils::{get_at_virtual_addr, PhysAddr},
    PageAllocator,
};

use super::platform_info::PlatformInfo;
use crate::println;

pub fn init_ioapic(platform_info: &PlatformInfo) {
    unsafe {
        for io_apic_info in &platform_info.apic.io_apics {
            let io_apic_address = crate::memory::PAGE_TREE_ALLOCATOR.allocate(Some(PhysAddr(io_apic_info.address.into())));
            let apic_registers_page_entry = crate::memory::PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(io_apic_address);
            apic_registers_page_entry.set_write_through_cahcing(true);
            apic_registers_page_entry.set_disable_cahce(true);
            core::arch::asm!(
                "mov rax, cr3",
                "mov cr3, rax",
                out("rax") _
            ); //clear the TLB
            let io_apic = get_at_virtual_addr::<IoApicRegisters>(io_apic_address);
            println!("got io apic");

            for i in 0..16 {
                let mut trigger_mode = 0;
                let mut polarity = 0;
                let mut gsi = i as u8;
                for interrupt_override in &platform_info.apic.interrupt_source_overrides {
                    if interrupt_override.source != i as u8 {
                        continue;
                    }
                    println!("{:#x?}", interrupt_override);
                    if let crate::acpi::madt::IntSoOverTriggerMode::LevelTriggered = interrupt_override.flags.trigger_mode() {
                        trigger_mode = 1;
                    }
                    if let crate::acpi::madt::IntSoOverPolarity::ActiveLow = interrupt_override.flags.polarity() {
                        polarity = 1;
                    }
                    gsi = interrupt_override.global_system_interrupt as u8;
                }

                let table_entry = RedTblEntry::new(
                    platform_info.boot_processor.apic_id,
                    0,
                    trigger_mode,
                    0,
                    polarity,
                    0,
                    0,
                    i as u8 + 32,
                );
                println!("setting entry {:b}", table_entry.0);
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
    pub fn get_version(&mut self) -> u32 {
        self.index = 1;
        self.data
    }

    pub fn get_id(&mut self) -> u32 {
        self.index = 0;
        self.data
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
