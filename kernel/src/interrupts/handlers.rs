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

//for now we rely on hopes and dreams that mmx and sse registers won't be used but it has to be
//fixed in the future. Same with the red zone

macro_rules! handler {
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                asm!(
                    "push rax",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",

                    "mov rdi, rsp",
                    "sub rsp, 8 //align the stack pointer",
                    "call {}",

                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rax",

                    "add rsp, 8 //undo stack pointer alignment",
                    "iretq",
                    sym $name,
                    options(noreturn)
                )

            }
        }
        wrapper
    }};
}

macro_rules! handler_with_error {
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                asm!(
                    "push rax",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",

                    "mov rsi, [rsp + 9*8] // load error code into rsi",
                    "mov rdi, rsp",
                    "add rdi, 10*8 //calcualte exception stack frame pointer",
                    "sub rsp, 8 //align the stack pointer",
                    "call {}",
                    "add rsp, 8 //undo stack pointer alignment",

                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rax",

                    "add rsp, 8 //pop error code",

                    "iretq",
                    sym $name,
                    options(noreturn)
                )
            }
        }
        wrapper
    }};
}

pub extern "C" fn divide_by_zero(stack_frame: &ExceptionStackFrame) {
    set_vga_text_foreground((0, 0, 255));
    println!("EXCEPTION: DIVIDE BY ZERO\n{:#?}", stack_frame);
    set_vga_text_foreground((255, 255, 255));
    loop {}
}

pub extern "C" fn invalid_opcode(stack_frame: &ExceptionStackFrame) {
    set_vga_text_foreground((0, 0, 255));
    println!(
        "nEXCEPTION: INVALID OPCODE at {:#x}\n{:#?}",
        stack_frame.instruction_pointer, stack_frame
    );
    set_vga_text_foreground((255, 255, 255));
    loop {}
}

pub extern "C" fn breakpoint(stack_frame: &ExceptionStackFrame) {
    set_vga_text_foreground((0, 255, 255));
    println!(
        "Breakpoint reached at {:#x}\n{:#?}",
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

pub extern "C" fn page_fault(stack_frame: &ExceptionStackFrame, error_code: u64) {
    set_vga_text_foreground((0, 0, 255));
    println!(
        "\nEXCEPTION: PAGE FAULT with error code\n{:#?}\n{:#?}",
        PageFaultErrorCode::from(error_code),
        stack_frame
    );
    set_vga_text_foreground((255, 255, 255));
}
