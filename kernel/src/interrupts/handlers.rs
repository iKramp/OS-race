use crate::println;
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
        "EXCEPTION: INVALID OPCODE at {:#X}\n{:#?}",
        stack_frame.instruction_pointer, stack_frame
    );
    set_vga_text_foreground((255, 255, 255));
    loop {}
}

pub extern "x86-interrupt" fn breakpoint(stack_frame: ExceptionStackFrame) {
    set_vga_text_foreground((0, 255, 255));
    println!(
        "Breakpoint reached at {:#X}\n{:#?}",
        stack_frame.instruction_pointer, stack_frame
    );
    set_vga_text_foreground((255, 255, 255));
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
    loop {}
}

pub extern "x86-interrupt" fn other_interrupt(_stack_frame: ExceptionStackFrame) {
    set_vga_text_foreground((0, 0, 255));
    println!("some interrupt");
    set_vga_text_foreground((255, 255, 255));
    loop {}
}
