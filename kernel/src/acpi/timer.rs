use core::u32;
use std::println;

use crate::{
    acpi::{cpu_locals, LAPIC_REGISTERS},
    handler,
    interrupts::{
        disable_pic_completely, handlers::apic_timer_tick, idt::{Entry, IDT}, APIC_TIMER_INIT, APIC_TIMER_TICKS, LEGACY_PIC_TIMER_TICKS, PIC_ACTUAL_FREQ, TIMER_DESIRED_FREQUENCY
    },
    proc::interrupt_context_switch,
};

use super::apic::LapicRegisters;

static mut TIMER_CONF: u32 = 0;
static mut INITIAL_COUNT: u32 = 0;

pub(super) fn activate_timer_ap(lapic_registers: &mut LapicRegisters) {
    unsafe {
        lapic_registers.lvt_timer.bytes = TIMER_CONF;
        lapic_registers.divide_configuration.bytes = 0;
        lapic_registers.initial_count.bytes = INITIAL_COUNT;
    }
}

pub(super) fn activate_timer(lapic_registers: &mut LapicRegisters) {
    //TODO: redo this logic, instead of waiting some ticks by lapic timer, wait 10 miliseconds by
    //legacy timer and set lapic timer divisor to very low
    let mut timer_conf = lapic_registers.lvt_timer.bytes;

    timer_conf &= !0xFF_u32;
    timer_conf |= 100; //init the timer vector //TODO reset
    timer_conf &= !(0b11 << 17);
    timer_conf |= 0b00 << 17; //set to oneshot
    timer_conf &= !(1 << 16); //unmask

    const TIMER_COUNT: u32 = u32::MAX;
    lapic_registers.lvt_timer.bytes = timer_conf;
    lapic_registers.divide_configuration.bytes = 0;
    lapic_registers.initial_count.bytes = TIMER_COUNT;

    let ticks;
    unsafe {
        let end_legacy_timer = LEGACY_PIC_TIMER_TICKS + PIC_ACTUAL_FREQ as u64 / 100; //10 ms
        #[allow(clippy::while_immutable_condition)] //timer mutates
        while LEGACY_PIC_TIMER_TICKS < end_legacy_timer {}
        ticks = lapic_registers.current_count.bytes;
        lapic_registers.initial_count.bytes = 0; //disable
        disable_pic_completely();
    }
    let ticks_counted = TIMER_COUNT - ticks;

    println!("Ticks: {}", ticks);

    unsafe { IDT.set(Entry::new(handler!(apic_timer_tick)), 32) };

    let initial_count = ticks_counted * 100 / TIMER_DESIRED_FREQUENCY;
    println!("Initial count: {} or {:x}", initial_count, initial_count);
    // for i in 0..70 {
    //     println!("");
    // }
    // panic!();

    timer_conf |= 0b01 << 17; // set to periodic
    timer_conf &= !0xFF_u32;
    timer_conf |= 32; //set correct interrupt vector
    lapic_registers.lvt_timer.bytes = timer_conf;
    lapic_registers.initial_count.bytes = initial_count as u32; //set to same frequency

    unsafe {
        TIMER_CONF = timer_conf;
        INITIAL_COUNT = initial_count as u32;
    }
}

pub fn time_since_boot() -> std::time::Duration {
    debug_assert!(unsafe { APIC_TIMER_INIT });
    let apic_id = cpu_locals::CpuLocals::get().apic_id;
    let time_seconds = unsafe { APIC_TIMER_TICKS.assume_init_ref()[apic_id as usize] };
    let timer_ticks_counted = unsafe { INITIAL_COUNT as u64 - LAPIC_REGISTERS.assume_init_ref().current_count.bytes as u64 };
    let time_nanos = unsafe { timer_ticks_counted * 1_000_000_000 / INITIAL_COUNT as u64 };
    std::time::Duration::new(time_seconds, time_nanos as u32)
}
