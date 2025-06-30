use core::time::Duration;
use std::{println, time::Instant};

use crate::{
    handler,
    interrupts::{
        TIMER_DESIRED_FREQUENCY,
        handlers::apic_timer_tick,
        idt::{Entry, IDT},
    },
    proc::interrupt_context_switch,
};

use super::apic::LapicRegisters;

static mut TIMER_CONF: u32 = 0;
static mut FREQUENCY: u64 = 0;

pub(super) fn setup_timer_ap(lapic_registers: &mut LapicRegisters) {
    unsafe {
        lapic_registers.lvt_timer.bytes = TIMER_CONF;
        lapic_registers.divide_configuration.bytes = 0;
        lapic_registers.initial_count.bytes = 0; //disable timer
    }
}

pub(super) fn activate_timer(lapic_registers: &mut LapicRegisters) {
    let mut timer_conf = unsafe { core::ptr::addr_of!(lapic_registers.lvt_timer.bytes).read_volatile() };

    timer_conf &= !0xFF_u32;
    timer_conf |= 255; //init the timer vector //TODO reset
    timer_conf &= !(0b11 << 17);
    timer_conf |= 0b00 << 17; //set to oneshot
    timer_conf &= !(1 << 16); //unmask

    const TIMER_COUNT: u32 = u32::MAX;
    let ticks;
    unsafe {
        core::ptr::addr_of_mut!(lapic_registers.lvt_timer.bytes).write_volatile(timer_conf);
        //no division
        core::ptr::addr_of_mut!(lapic_registers.divide_configuration.bytes).write_volatile(0b1011);
        core::ptr::addr_of_mut!(lapic_registers.initial_count.bytes).write_volatile(TIMER_COUNT);

        let start_time = Instant::now();
        let end_time = start_time + Duration::from_millis(5);
        while Instant::now() < end_time {}

        ticks = core::ptr::addr_of!(lapic_registers.current_count.bytes).read_volatile();
        core::ptr::addr_of_mut!(lapic_registers.initial_count.bytes).write_volatile(0); //disable
        crate::interrupts::trigger_pit_eoi();
    }

    let ticks_counted = TIMER_COUNT - ticks;
    let frequency = ticks_counted as u64 * 1_000 / 5; //ticks counted in 5 miliseconds

    println!("Ticks: {}", ticks);

    unsafe { IDT.set(Entry::new(handler!(apic_timer_tick)), 32) };

    let initial_count = ticks_counted * 100 / TIMER_DESIRED_FREQUENCY;
    println!("Initial count: {} or {:x}", initial_count, initial_count);

    timer_conf &= !0xFF_u32;
    timer_conf |= 32; //set correct interrupt vector
    unsafe {
        core::ptr::addr_of_mut!(lapic_registers.lvt_timer.bytes).write_volatile(timer_conf);
        core::ptr::addr_of_mut!(lapic_registers.initial_count.bytes).write_volatile(0);
    }

    unsafe {
        TIMER_CONF = timer_conf;
        FREQUENCY = frequency;
        std::thread::SLEEP = sleep_duration;
    }
}

fn sleep_duration(duration: Duration) {
    if duration.as_micros() < 1 {
        return; //no need to sleep
    }
    set_timeout(duration);
    unsafe { core::arch::asm!("hlt") };
}

pub fn set_timeout(duration: Duration) {
    let seconds = duration.as_secs();
    let nanos = duration.subsec_nanos() as u64;
    // divde by | config in division register
    // 2        | 0b0000
    // 4        | 0b0001
    // 8        | 0b0010
    // 16       | 0b0011
    // 32       | 0b1000
    // 64       | 0b1001
    // 128      | 0b1010
    // 1        | 0b1011

    let tics_seconds = seconds * (unsafe { FREQUENCY });
    let ticks_nanos = nanos * (unsafe { FREQUENCY }) / 1_000_000_000;
    let ticks = tics_seconds + ticks_nanos;
    let leading_zeros = ticks.leading_zeros();
    let (division, ticks) = match leading_zeros {
        32.. => (0b1011, ticks),        //no division
        31 => (0b0000, ticks / 2),      //divide by 2
        30 => (0b0001, ticks / 4),      //divide by 4
        29 => (0b0010, ticks / 8),      //divide by 8
        28 => (0b0011, ticks / 16),     //divide by 16
        27 => (0b1000, ticks / 32),     //divide by 32
        26 => (0b1001, ticks / 64),     //divide by 64
        25 => (0b1010, ticks / 128),    //divide by 128
        _ => (0b1010, u32::MAX as u64), //more than 10 minutes timeout, treat as max
    };
    let lapic_registers = unsafe { super::LAPIC_REGISTERS.assume_init_mut() };
    unsafe {
        core::ptr::addr_of_mut!(lapic_registers.divide_configuration.bytes).write_volatile(division);
        core::ptr::addr_of_mut!(lapic_registers.initial_count.bytes).write_volatile(ticks as u32);
    }
}
