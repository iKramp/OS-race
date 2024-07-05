use std::{mem_utils::VirtAddr, PageAllocator};

use crate::{
    interrupts::{LEGACY_PIC_TIMER_TICKS, PIC_TIMER_FREQUENCY, TIMER_TICKS},
    println,
    utils::byte_to_port,
};

pub static mut LAPIC_REGISTERS: VirtAddr = VirtAddr(0);
const USE_LEGACY_TIMER: bool = false;

pub fn enable_apic(platform_info: &super::platform_info::PlatformInfo) {
    unsafe {
        LAPIC_REGISTERS = crate::memory::PAGE_TREE_ALLOCATOR.allocate(Some(platform_info.apic.lapic_address));
        let apic_registers_page_entry = crate::memory::PAGE_TREE_ALLOCATOR.get_page_table_entry_mut(LAPIC_REGISTERS);
        apic_registers_page_entry.set_write_through_cahcing(true);
        apic_registers_page_entry.set_disable_cahce(true);
        core::arch::asm!(
            "mov rax, cr3",
            "mov cr3, rax",
            out("rax") _
        ); //clear the TLB
        let lapic_registers = std::mem_utils::get_at_virtual_addr::<LapicRegisters>(LAPIC_REGISTERS);

        //init APIC

        activate_timer(lapic_registers);
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
    const DIVIDE_VALUE: u32 = 16; //could be 1 on real PCs but VMs don't like it
    lapic_registers.lvt_timer.bytes = timer_conf;
    lapic_registers.divide_configuration.bytes = DIVIDE_VALUE;
    lapic_registers.initial_count.bytes = TIMER_COUNT;

    let ticks;
    unsafe {
        let start_legacy_timer = LEGACY_PIC_TIMER_TICKS;
        while TIMER_TICKS == 0 {}
        ticks = LEGACY_PIC_TIMER_TICKS - start_legacy_timer;
    }
    disable_pic();
    if USE_LEGACY_TIMER {
        timer_conf |= 1 << 16; //mask
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

    //INFO: I GIVE UP IT'S IMPOSSIBLE TO MAKE A RELIABLE TIMER IN A VM, I'LL JUST USE THE LEGACY PIT
}

pub fn disable_pic() {
    const PIC1_DATA: u16 = 0x21;
    const PIC2_DATA: u16 = 0xA1;

    let mask = if USE_LEGACY_TIMER { 0xFE } else { 0xFF };

    byte_to_port(PIC1_DATA, mask); //mask interrupts
    byte_to_port(PIC2_DATA, mask);
}

#[repr(C, packed)]
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
    error_status: LapicRegisterValueStructure,
    reserved_6: LapicRegisterValueStructure,
    reserved_7: LapicRegisterValueStructure,
    reserved_8: LapicRegisterValueStructure,
    reserved_9: LapicRegisterValueStructure,
    reserved_10: LapicRegisterValueStructure,
    reserved_11: LapicRegisterValueStructure,
    lvt_corrected_machine_check_interrupt: LapicRegisterValueStructure,
    interurpt_command_register: TwoDWordStructure,
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

#[repr(C, packed)]
pub struct LapicRegisterValueStructure {
    pub bytes: u32,
    padding_0: u32,
    padding_1: u64,
}

#[repr(C, packed)]
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

#[repr(C, packed)]
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
