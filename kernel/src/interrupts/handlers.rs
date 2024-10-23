use crate::println;
use crate::utils::{byte_form_port, byte_to_port};
use crate::vga::vga_text::set_vga_text_foreground;
#[allow(unused_imports)] //they are used in macros
use core::arch::asm;

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

pub extern "x86-interrupt" fn invalid_opcode(stack_frame: ExceptionStackFrame) -> ! {
    set_vga_text_foreground((0, 0, 255));
    println!(
        "EXCEPTION: INVALID OPCODE at {:#X}\n{:#X?}",
        stack_frame.instruction_pointer, stack_frame
    );
    set_vga_text_foreground((255, 255, 255));
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}

pub extern "x86-interrupt" fn breakpoint(stack_frame: ExceptionStackFrame) {
    let continue_executing = false;
    set_vga_text_foreground((0, 255, 255));
    println!(
        "Breakpoint reached at {:#X}",
        stack_frame.instruction_pointer
    );
    set_vga_text_foreground((255, 255, 255));
    loop {
        if continue_executing {
            break;
        }
    }
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

pub extern "x86-interrupt" fn page_fault(stack_frame: ExceptionStackFrame, error_code: u64) -> ! {
    set_vga_text_foreground((0, 0, 255));
    println!(
        "EXCEPTION: PAGE FAULT with error code\n{:#X?}\n{:#X?}",
        PageFaultErrorCode::from(error_code),
        stack_frame
    );
    set_vga_text_foreground((255, 255, 255));
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}

//gpf
pub extern "x86-interrupt" fn general_protection_fault(stack_frame: ExceptionStackFrame, error_code: u64) -> ! {
    set_vga_text_foreground((0, 0, 255));
    println!("EXCEPTION: GPF\n{:#X?}\n{:#x?}", stack_frame, error_code);
    set_vga_text_foreground((255, 255, 255));
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}

pub extern "x86-interrupt" fn other_legacy_interrupt(_stack_frame: ExceptionStackFrame) {
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

pub extern "x86-interrupt" fn other_apic_interrupt(_stack_frame: ExceptionStackFrame) {
    apic_eoi();
}

pub extern "x86-interrupt" fn apic_timer_tick(_stack_frame: ExceptionStackFrame) {
    unsafe {
        super::TIMER_TICKS += 1;
        apic_eoi();
    }
}

pub extern "x86-interrupt" fn legacy_timer_tick_testing(_stack_frame: ExceptionStackFrame) {
    unsafe {
        super::LEGACY_PIC_TIMER_TICKS += 1;
    }
    legacy_eoi();
}

pub extern "x86-interrupt" fn legacy_timer_tick(_stack_frame: ExceptionStackFrame) {
    unsafe {
        super::TIMER_TICKS += 1;
    }
    legacy_eoi();
}

pub extern "x86-interrupt" fn apic_error(_stack_frame: ExceptionStackFrame) {
    unsafe {
        let lapic_registers = std::mem_utils::get_at_virtual_addr::<crate::acpi::LapicRegisters>(crate::acpi::LAPIC_REGISTERS);
        lapic_registers.error_status.bytes = 0; //activate it to load the real value
        let _error_register = &lapic_registers.error_status;
        //do error shit
        apic_eoi();
    }
}

pub extern "x86-interrupt" fn spurious_interrupt(_stack_frame: ExceptionStackFrame) {
    apic_eoi();
}

pub extern "x86-interrupt" fn legacy_keyboard_interrupt(_stack_frame: ExceptionStackFrame) {
    let code = byte_form_port(0x60);
    //println!("{code}");
    crate::keyboard::handle_key(code);
    legacy_eoi();
}

pub extern "x86-interrupt" fn apic_keyboard_interrupt(_stack_frame: ExceptionStackFrame) {
    apic_eoi();
    let code = byte_form_port(0x60);
    crate::keyboard::handle_key(code);
    //println!("{code}");
}

pub extern "x86-interrupt" fn ps2_mouse_interrupt(_stack_frame: ExceptionStackFrame) {
    apic_eoi();
}

pub extern "x86-interrupt" fn fpu_interrupt(_stack_frame: ExceptionStackFrame) {
    apic_eoi();
}

pub extern "x86-interrupt" fn primary_ata_hard_disk(_stack_frame: ExceptionStackFrame) {
    apic_eoi();
}
