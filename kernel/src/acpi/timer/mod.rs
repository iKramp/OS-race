use core::mem::MaybeUninit;
use std::{println, time::Instant};

pub(super) mod hpet;
mod rtc;
mod tsc;

pub(super) static mut HPET_ACPI_TABLE: MaybeUninit<&hpet::HpetTable> = MaybeUninit::uninit();

trait Timer {
    fn start(&self, now: Instant) -> bool;
    fn get_time(&self) -> Instant;
}

pub fn init() {
    let now = rtc::RTC_WRAPPER.get_time();
    println!("Current time: {:?}", now);
    let tsc_success = unsafe { tsc::TSC_WRAPPER.start(now) };
    if !tsc_success {
        panic!("HPET not yet ready to use");
    }
    let now_tsc = unsafe { tsc::TSC_WRAPPER.get_time() };
    println!("Current time (TSC): {:?}", now_tsc);
    // let _success = unsafe { hpet::HPET.start(now) };
}
