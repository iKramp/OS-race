#![allow(clippy::unusual_byte_groupings, static_mut_refs)]

use std::{
    eh::int3,
    mem_utils::{PhysAddr, VirtAddr},
    PageAllocator,
};

use crate::{
    interrupts::handlers::*,
    interrupts::idt::{Entry, IDT},
    interrupts::{LEGACY_PIC_TIMER_TICKS, PIC_TIMER_FREQUENCY, TIMER_TICKS},
    println,
    utils::byte_to_port,
};

pub static mut LAPIC_REGISTERS: VirtAddr = VirtAddr(0);
const USE_LEGACY_TIMER: bool = false;
const DIVIDE_VALUE: u32 = 16; //could be 1 on real PCs but VMs don't like it

pub fn enable_apic(platform_info: &super::platform_info::PlatformInfo, processor_id: u8) {
    let bsp = processor_id == platform_info.boot_processor.processor_id;
    if bsp {
        disable_pic_keep_timer()
    };
    let lapic_registers = get_lapic_registers(platform_info.apic.lapic_address, bsp);

    if bsp {
        unsafe {
            IDT.set(Entry::converging(other_apic_interrupt), 64);
            IDT.set(Entry::converging(other_apic_interrupt), 65);
            IDT.set(Entry::converging(other_apic_interrupt), 66);
            IDT.set(Entry::converging(apic_error), 67);
            IDT.set(Entry::converging(other_apic_interrupt), 68);
            IDT.set(Entry::converging(other_apic_interrupt), 69);
        }
    }

    lapic_registers.lvt_corrected_machine_check_interrupt.bytes = 0b00000000_00000000_0_000_0_0_000_01000000_u32;
    lapic_registers.lvt_lint0.bytes = 0b00000000_00000000_0_000_0_0_000_01000001_u32;
    lapic_registers.lvt_lint1.bytes = 0b00000000_00000000_0_000_0_0_000_01000010_u32;
    lapic_registers.lvt_error.bytes = 0b00000000_00000000_0_000_0_0_000_01000011_u32;
    lapic_registers.lvt_performance_monitoring_counters.bytes = 0b00000000_00000000_0_000_0_0_000_01000100_u32;
    lapic_registers.lvt_thermal_sensor.bytes = 0b00000000_00000000_0_000_0_0_000_01000101_u32;

    let mut nmi_source = 0b00000000_00000000_0_000_0_0_100_00000000_u32;
    let lapic_nmi = platform_info.get_nmi_structure(processor_id);

    if let crate::acpi::madt::IntSoOverTriggerMode::LevelTriggered = lapic_nmi.flags.trigger_mode() {
        nmi_source |= 1 << 15;
    }
    if let crate::acpi::madt::IntSoOverPolarity::ActiveLow = lapic_nmi.flags.polarity() {
        nmi_source |= 1 << 13;
    }
    if lapic_nmi.lint == 0 {
        lapic_registers.lvt_lint0.bytes = nmi_source;
    } else {
        lapic_registers.lvt_lint1.bytes = nmi_source;
    }

    //fully enable apic:
    lapic_registers.spurious_interrupt.bytes = 0b0000000000000000000_0_00_0_1_11111111_u32;
    lapic_registers.task_priority.bytes = 0;

    if !bsp {
        activate_timer_ap(lapic_registers);
        return;
    }
    activate_timer(lapic_registers);

    unsafe {
        for i in 38..255 {
            IDT.set(Entry::converging(other_apic_interrupt), i);
        }

        IDT.set(Entry::converging(apic_keyboard_interrupt), 32 + 1);
        IDT.set(Entry::converging(ps2_mouse_interrupt), 32 + 12);
        IDT.set(Entry::converging(fpu_interrupt), 32 + 13);
        IDT.set(Entry::converging(primary_ata_hard_disk), 32 + 14);
    }
    disable_pic_completely();
}

fn get_lapic_registers(lapic_address: PhysAddr, bsp: bool) -> &'static mut LapicRegisters {
    unsafe {
        if bsp {
            LAPIC_REGISTERS = crate::memory::PAGE_TREE_ALLOCATOR.allocate(Some(lapic_address));
            let apic_registers_page_entry = crate::memory::PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(LAPIC_REGISTERS);
            apic_registers_page_entry.set_write_through_cahcing(true);
            apic_registers_page_entry.set_disable_cahce(true);
            core::arch::asm!(
                "mov rax, cr3",
                "mov cr3, rax",
                out("rax") _
            ); //clear the TLB
        }
        std::mem_utils::get_at_virtual_addr::<LapicRegisters>(LAPIC_REGISTERS)
    }
}

static mut TIMER_CONF: u32 = 0;
static mut INITIAL_COUNT: u32 = 0;

fn activate_timer_ap(lapic_registers: &mut LapicRegisters) {
    unsafe {
        lapic_registers.lvt_timer.bytes = TIMER_CONF;
        lapic_registers.divide_configuration.bytes = DIVIDE_VALUE;
        lapic_registers.initial_count.bytes = INITIAL_COUNT;
    }
}

fn activate_timer(lapic_registers: &mut LapicRegisters) {
    let mut timer_conf = lapic_registers.lvt_timer.bytes;

    timer_conf &= !0xFF_u32;
    timer_conf |= 100; //init the timer vector //TODO reset
    timer_conf &= !(0b11 << 17);
    timer_conf |= 0b00 << 17; //set to one-shot
    timer_conf &= !(1 << 16); //unmask

    const TIMER_COUNT: u32 = 100000000;
    lapic_registers.lvt_timer.bytes = timer_conf;
    lapic_registers.divide_configuration.bytes = DIVIDE_VALUE;
    lapic_registers.initial_count.bytes = TIMER_COUNT;

    let ticks;
    unsafe {
        let start_legacy_timer = LEGACY_PIC_TIMER_TICKS;
        while TIMER_TICKS == 0 {}
        ticks = LEGACY_PIC_TIMER_TICKS - start_legacy_timer;
    }

    if USE_LEGACY_TIMER {
        timer_conf |= 1 << 16; //mask
        unsafe { IDT.set(Entry::converging(legacy_timer_tick), 32) };
    } else {
        disable_pic_completely();
        unsafe { IDT.set(Entry::converging(apic_timer_tick), 32) };
    }

    timer_conf |= 0b01 << 17; // set to periodic
    timer_conf &= !0xFF_u32;
    timer_conf |= 32; //set correct interrupt vector
    lapic_registers.lvt_timer.bytes = timer_conf;
    lapic_registers.initial_count.bytes = TIMER_COUNT / ticks as u32; //set to same frequency
    let frequency = TIMER_COUNT as u64 * DIVIDE_VALUE as u64 * PIC_TIMER_FREQUENCY as u64 / ticks;
    println!("APIC timer is running at {} Hz", frequency);
    println!(
        "The selected timer is running and producing ticks at {} Hz",
        PIC_TIMER_FREQUENCY
    );

    unsafe {
        TIMER_CONF = lapic_registers.lvt_timer.bytes;
        INITIAL_COUNT = lapic_registers.initial_count.bytes;
    }
}

pub fn disable_pic_keep_timer() {
    const PIC1_DATA: u16 = 0x21;
    const PIC2_DATA: u16 = 0xA1;

    byte_to_port(PIC1_DATA, 0xFE); //mask interrupts, keep timer
    byte_to_port(PIC2_DATA, 0xFE);

    byte_to_port(PIC1_DATA - 1, 0x20); //trigger EOI
    byte_to_port(PIC2_DATA - 1, 0x20);

    //disconnect_imcr();
}

pub fn disable_pic_completely() {
    const PIC1_DATA: u16 = 0x21;
    const PIC2_DATA: u16 = 0xA1;

    byte_to_port(PIC1_DATA, 0xFF); //mask interrupts
    byte_to_port(PIC2_DATA, 0xFF);

    byte_to_port(PIC1_DATA - 1, 0x20); //trigger EOI
    byte_to_port(PIC2_DATA - 1, 0x20);

    disconnect_imcr();
}

fn disconnect_imcr() {
    const IMCR: u16 = 0x22;

    byte_to_port(IMCR, 0x70);
    byte_to_port(IMCR + 1, 0x01);
}

#[repr(C)]
pub struct LapicRegisters {
    reserved_0: LapicRegisterValueStructure,
    reserved_1: LapicRegisterValueStructure,
    lapic_id: LapicRegisterValueStructure,
    lapic_version: LapicRegisterValueStructure,
    reserved_2: LapicRegisterValueStructure,
    reserved_3: LapicRegisterValueStructure,
    reserved_4: LapicRegisterValueStructure,
    reserved_5: LapicRegisterValueStructure,
    task_priority: LapicRegisterValueStructure,
    arbitration_proority: LapicRegisterValueStructure,
    processor_priority: LapicRegisterValueStructure,
    pub end_of_interrupt: LapicRegisterValueStructure,
    remote_read: LapicRegisterValueStructure,
    logical_destination: LapicRegisterValueStructure,
    destination_format: LapicRegisterValueStructure,
    spurious_interrupt: LapicRegisterValueStructure,
    in_service: EightDWordStructure,
    trigger_mode: EightDWordStructure,
    interrupt_request: EightDWordStructure,
    pub error_status: LapicRegisterValueStructure,
    reserved_6: LapicRegisterValueStructure,
    reserved_7: LapicRegisterValueStructure,
    reserved_8: LapicRegisterValueStructure,
    reserved_9: LapicRegisterValueStructure,
    reserved_10: LapicRegisterValueStructure,
    reserved_11: LapicRegisterValueStructure,
    lvt_corrected_machine_check_interrupt: LapicRegisterValueStructure,
    interrupt_command_register_0_32: LapicRegisterValueStructure,
    interrupt_command_register_32_64: LapicRegisterValueStructure,
    lvt_timer: LapicRegisterValueStructure,
    lvt_thermal_sensor: LapicRegisterValueStructure,
    lvt_performance_monitoring_counters: LapicRegisterValueStructure,
    lvt_lint0: LapicRegisterValueStructure,
    lvt_lint1: LapicRegisterValueStructure,
    lvt_error: LapicRegisterValueStructure,
    initial_count: LapicRegisterValueStructure,
    current_count: LapicRegisterValueStructure,
    reserved_12: LapicRegisterValueStructure,
    reserved_13: LapicRegisterValueStructure,
    reserved_14: LapicRegisterValueStructure,
    reserved_15: LapicRegisterValueStructure,
    divide_configuration: LapicRegisterValueStructure,
    reserved_16: LapicRegisterValueStructure,
}

impl LapicRegisters {
    pub fn send_ipi(&mut self, delivery_mode: u8, destination: u8, vector: u8) {
        unsafe {
            (&mut self.interrupt_command_register_32_64.bytes as *mut u32).write_volatile((destination as u32) << 24);
            (&mut self.interrupt_command_register_0_32.bytes as *mut u32)
                .write_volatile((vector as u32) | ((delivery_mode as u32) << 8));
        }
    }

    pub fn send_init_ipi(&mut self, destination: u8) {
        self.send_ipi(0b101, destination, 0);
    }

    pub fn send_startup_ipi(&mut self, destination: u8, start_page: u8) {
        self.send_ipi(0b110, destination, start_page);
    }
}

#[repr(C)]
pub struct LapicRegisterValueStructure {
    pub bytes: u32,
    padding_0: u32,
    padding_1: u64,
}

#[repr(C)]
struct EightDWordStructure {
    bits_000_031: LapicRegisterValueStructure,
    bits_032_063: LapicRegisterValueStructure,
    bits_064_095: LapicRegisterValueStructure,
    bits_096_127: LapicRegisterValueStructure,
    bits_128_159: LapicRegisterValueStructure,
    bits_160_191: LapicRegisterValueStructure,
    bits_192_223: LapicRegisterValueStructure,
    bits_224_255: LapicRegisterValueStructure,
}

#[allow(clippy::identity_op)]
impl EightDWordStructure {
    pub fn write(&mut self, low: u128, high: u128) {
        self.bits_000_031.bytes = ((low >> 00) & 0xFFFFFFFF) as u32;
        self.bits_032_063.bytes = ((low >> 32) & 0xFFFFFFFF) as u32;
        self.bits_064_095.bytes = ((low >> 64) & 0xFFFFFFFF) as u32;
        self.bits_096_127.bytes = ((low >> 96) & 0xFFFFFFFF) as u32;
        self.bits_128_159.bytes = ((high >> 00) & 0xFFFFFFFF) as u32;
        self.bits_160_191.bytes = ((high >> 32) & 0xFFFFFFFF) as u32;
        self.bits_192_223.bytes = ((high >> 64) & 0xFFFFFFFF) as u32;
        self.bits_224_255.bytes = ((high >> 96) & 0xFFFFFFFF) as u32;
    }

    #[allow(clippy::identity_op)]
    pub fn read(&self) -> (u128, u128) {
        (
            ((self.bits_000_031.bytes as u128) << 00)
                | ((self.bits_032_063.bytes as u128) << 32)
                | ((self.bits_064_095.bytes as u128) << 64)
                | ((self.bits_096_127.bytes as u128) << 96),
            ((self.bits_128_159.bytes as u128) << 00)
                | ((self.bits_160_191.bytes as u128) << 32)
                | ((self.bits_192_223.bytes as u128) << 64)
                | ((self.bits_224_255.bytes as u128) << 96),
        )
    }
}

#[repr(C)]
struct TwoDWordStructure {
    pub bits_000_031: LapicRegisterValueStructure,
    pub bits_032_063: LapicRegisterValueStructure,
}

impl TwoDWordStructure {
    pub fn write(&mut self, val: u64) {
        self.bits_000_031.bytes = (val & 0xFFFFFFFF) as u32;
        self.bits_000_031.bytes = (val & 0xFFFFFFFF) as u32;
    }

    pub fn read(&self) -> u64 {
        self.bits_000_031.bytes as u64 | ((self.bits_032_063.bytes as u64) << 32)
    }
}
