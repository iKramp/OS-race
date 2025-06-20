use core::mem::MaybeUninit;
use std::time::Instant;

pub(super) mod hpet;
mod tsc;

pub(super) static mut HPET_ACPI_TABLE: MaybeUninit<&hpet::HpetTable> = MaybeUninit::uninit();

trait Timer {
    fn start(&self, now: Instant) -> bool;
    fn get_time() -> Instant;
}

pub fn init() {
    let _tsc_success = unsafe { tsc::TSC_WRAPPER.start(std::time::UNIX_EPOCH) };
    let _success = unsafe { hpet::HPET.start(std::time::UNIX_EPOCH) };
}
