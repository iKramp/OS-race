use core::arch::asm;
use std::printlnc;

use crate::{
    cpuid,
    interrupts::{LEGACY_PIC_TIMER_TICKS, PIC_ACTUAL_FREQ},
};

use super::Timer;

pub(super) struct TscWrapper {
    start: std::time::Instant,
    ticks_on_start: u64,
    ticks_per_second: u64,
}

pub(super) static mut TSC_WRAPPER: TscWrapper = TscWrapper {
    start: std::time::UNIX_EPOCH,
    ticks_on_start: 0,
    ticks_per_second: 0,
};

impl TscWrapper {
    pub fn get_ticks() -> u64 {
        // Read the Time Stamp Counter (TSC) using inline assembly
        let ticks_eax: u32;
        let ticks_edx: u32;
        unsafe {
            asm!("rdtsc", out("eax") ticks_eax, out("edx") ticks_edx, options(nomem, nostack));
        }
        ((ticks_edx as u64) << 32) | (ticks_eax as u64)
    }
}

impl Timer for TscWrapper {
    fn start(&self, now: std::time::Instant) -> bool {
        //check availability of TSC
        let leaf_1_edx = if let Some(leaf) = cpuid::get_cpuid_leaf(1) {
            leaf.edx
        } else {
            printlnc!((255, 0, 0), "TSC: CPUID leaf 1 not supported by CPU");
            return false;
        };
        let tsc_exists = leaf_1_edx & (1 << 4) != 0;
        if !tsc_exists {
            printlnc!((255, 0, 0), "TSC: not supported by CPU");
            return false;
        }
        let leaf_0x80000007_edx = if let Some(leaf) = cpuid::get_cpuid_leaf(0x80000007) {
            leaf.edx
        } else {
            printlnc!((255, 0, 0), "TSC: CPUID leaf 0x80000007 not supported by CPU");
            return false;
        };
        let tsc_is_invariant = leaf_0x80000007_edx & (1 << 8) != 0;
        if !tsc_is_invariant {
            printlnc!((0, 0, 255), "TSC: not invariant");
            return false;
        }

        let tsc_start;
        unsafe {
            let end_legacy_timer = LEGACY_PIC_TIMER_TICKS + PIC_ACTUAL_FREQ as u64 / 1000; //1 ms
            tsc_start = TscWrapper::get_ticks();
            #[allow(clippy::while_immutable_condition)] //timer mutates
            while LEGACY_PIC_TIMER_TICKS < end_legacy_timer {}
            let tsc_end = TscWrapper::get_ticks();
            let ticks_counted = tsc_end - tsc_start;
            TSC_WRAPPER.ticks_per_second = ticks_counted * 1000;
        }
        unsafe { 
            TSC_WRAPPER.start = now;
            TSC_WRAPPER.ticks_on_start = tsc_start;
        }

        true
    }

    fn get_time(&self) -> std::time::Instant {
        let ticks = TscWrapper::get_ticks();
        let tps = unsafe { TSC_WRAPPER.ticks_per_second };
        let elapsed = ticks - tps;
        let seconds = elapsed / tps;
        let secons_ticks = seconds * tps;
        let nanos = ((elapsed - secons_ticks) * 1_000_000_000) / tps;
        let since_start = core::time::Duration::new(seconds, nanos as u32);
        unsafe { TSC_WRAPPER.start + since_start }
    }
}
