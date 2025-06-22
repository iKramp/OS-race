use std::println;

use crate::{
    acpi::{LAPIC_REGISTERS, cpu_locals},
    handler,
    interrupts::{
        APIC_TIMER_INIT, TIMER_DESIRED_FREQUENCY,
        disable_pic_completely,
        handlers::apic_timer_tick,
        idt::{Entry, IDT},
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

    let end_time = crate::clocks::get_time() + std::time::Duration::from_millis(5);
    
    while crate::clocks::get_time() < end_time {}

    let ticks = lapic_registers.current_count.bytes;

    lapic_registers.initial_count.bytes = 0; //disable
    disable_pic_completely();
    let ticks_counted = TIMER_COUNT - ticks;

    println!("Ticks: {}", ticks);

    unsafe { IDT.set(Entry::new(handler!(apic_timer_tick)), 32) };

    let initial_count = ticks_counted * 100 / TIMER_DESIRED_FREQUENCY;
    println!("Initial count: {} or {:x}", initial_count, initial_count);
    // for i in 0..70 {
    //     println!("");
    // }
    // panic!();

    timer_conf &= !0xFF_u32;
    timer_conf |= 32; //set correct interrupt vector
    lapic_registers.lvt_timer.bytes = timer_conf;
    lapic_registers.initial_count.bytes = initial_count; //set to same frequency

    unsafe {
        TIMER_CONF = timer_conf;
        INITIAL_COUNT = initial_count;
    }
}
