use crate::utils::{byte_from_port, byte_to_port};
#[allow(unused_imports)] //they are used in macros
use core::arch::asm;
use std::printlnc;

use super::macros::ProcData;

pub extern "C" fn invalid_opcode(proc_data: &mut ProcData) {
    printlnc!(
        (0, 0, 255),
        "EXCEPTION: INVALID OPCODE at {:#X}:{:#X}",
        proc_data.cs,
        proc_data.rip
    );
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}

pub extern "C" fn breakpoint(proc_data: &mut ProcData) {
    printlnc!((0, 255, 255), "Breakpoint reached at {:#X}:{:#X}", proc_data.cs, proc_data.rip);
    apic_eoi();
    legacy_eoi();
}

#[derive(Debug)]
#[allow(dead_code)] //not actually dead, is used in println
struct PageFaultErrorCode {
    protection_violation: bool,
    caused_by_write: bool,
    user_mode: bool,
    malformed_table: bool,
    instruction_fetch: bool,
}

impl From<u64> for PageFaultErrorCode {
    fn from(value: u64) -> Self {
        Self {
            protection_violation: value & (1 << 0) != 0,
            caused_by_write: value & (1 << 1) != 0,
            user_mode: value & (1 << 2) != 0,
            malformed_table: value & (1 << 3) != 0,
            instruction_fetch: value & (1 << 4) != 0,
        }
    }
}

pub extern "C" fn page_fault(proc_data: &mut ProcData) {
    printlnc!(
        (0, 0, 255),
        "EXCEPTION: PAGE FAULT. error code: {:#X?}",
        PageFaultErrorCode::from(proc_data.err_code),
    );
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}

//gpf
pub extern "C" fn general_protection_fault(proc_data: &mut ProcData) {
    printlnc!((0, 0, 255), "EXCEPTION: GPF. err code: {:#X?}", proc_data.err_code);
    printlnc!((0, 0, 255), "EXCEPTION: GPF. proc_data: {:#X?}", proc_data);
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}

pub extern "C" fn other_legacy_interrupt(_proc_data: &mut ProcData) {
    printlnc!((0, 0, 255), "interrupt: OTHER LEGACY INTERRUPT");
    legacy_eoi();
}

#[inline]
pub fn apic_eoi() {
    unsafe {
        let lapic_registers = std::mem_utils::get_at_virtual_addr::<crate::acpi::LapicRegisters>(crate::acpi::LAPIC_REGISTERS);
        lapic_registers.end_of_interrupt.bytes = 0;
    }
}

#[inline]
fn legacy_eoi() {
    byte_to_port(0x20, 0x20);
}

pub extern "C" fn other_apic_interrupt(_proc_data: &mut ProcData) {
    apic_eoi();
}

pub extern "C" fn apic_timer_tick(_proc_data: &mut ProcData) {
    unsafe {
        super::TIMER_TICKS += 1;
        apic_eoi();
    }
}

pub extern "C" fn legacy_timer_tick_testing(_proc_data: &mut ProcData) {
    unsafe {
        super::LEGACY_PIC_TIMER_TICKS += 1;
    }
    legacy_eoi();
}

pub extern "C" fn legacy_timer_tick(_proc_data: &mut ProcData) {
    unsafe {
        super::TIMER_TICKS += 1;
    }
    legacy_eoi();
}

pub extern "C" fn apic_error(_proc_data: &mut ProcData) {
    unsafe {
        let lapic_registers = std::mem_utils::get_at_virtual_addr::<crate::acpi::LapicRegisters>(crate::acpi::LAPIC_REGISTERS);
        lapic_registers.error_status.bytes = 0; //activate it to load the real value
        let _error_register = &lapic_registers.error_status;
        //do error shit
        apic_eoi();
    }
}

pub extern "C" fn spurious_interrupt(_proc_data: &mut ProcData) {
    apic_eoi();
}

pub extern "C" fn legacy_keyboard_interrupt(_proc_data: &mut ProcData) {
    let code = byte_from_port(0x60);
    //println!("{code}");
    crate::keyboard::handle_key(code);
    legacy_eoi();
}

pub extern "C" fn apic_keyboard_interrupt(_proc_data: &mut ProcData) {
    apic_eoi();
    let code = byte_from_port(0x60);
    crate::keyboard::handle_key(code);
    //println!("{code}");
}

pub extern "C" fn ps2_mouse_interrupt(_proc_data: &mut ProcData) {
    apic_eoi();
}

pub extern "C" fn fpu_interrupt(_proc_data: &mut ProcData) {
    apic_eoi();
}

pub extern "C" fn primary_ata_hard_disk(_proc_data: &mut ProcData) {
    apic_eoi();
}


