use crate::proc::interrupt_context_switch;

use super::{disable_interrupts, enable_interrupts};

#[macro_export]
macro_rules! handler {
    (
        $name:ident $(, $flag:ident )* $(,)?
    ) => {{
        #[unsafe(naked)]
        extern "C" fn wrapper() -> ! {
            //TODO: any kind of change here should be matched with the one in dispatcher.rs
            core::arch::naked_asm!(
                //potentially padding, but we always start with a clean stack if we're the first
                //level. IS aligned to 16 bytes
                //
                //pre-pushed values
                //
                //ss (64 bit)
                //rsp
                //rflags
                //cs (64 bit)
                //rip
                //
                //possibly error code

                handler!(@if_not_flag has_code, $($flag)* {
                    "push 0"
                }),

                //save all general purpose registers
                "push rax",
                "push rbx",
                "push rcx",
                "push rdx",
                "push rsi",
                "push rdi",
                "push rbp",
                //not pushing stack pointer, that's already changed
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "push r12",
                "push r13",
                "push r14",
                "push r15",


                handler!(@if_else_flag slow_swap, $($flag)*, {
                    "
                        mov ebx, 1 //idk some flag linux sets
                        mov ecx, 0xc0000101 //gs_base
                        rdmsr
                        test edx, edx
                        js 3f

                        swapgs
                        push 0x1 //swapped
                        jmp 4f
                        3:
                        push 0x0 //not swapped
                        4:
                    "
                } {
                    //mov pushed cs to register by offsetting from rsp
                    "
                        xor rax, rax
                        mov rax, [rsp + 8 * 17] //cs
                                                //skip swap if rax == 8
                        cmp rax, 8
                        je 3f

                        swapgs
                        push 0x1 //swapped
                        jmp 4f
                        3:
                        push 0x0 //not swapped
                        4:
                    "
                }),

                "mov rdi, rsp",
                "add rdi, 8", //start of proc data

                handler!(@if_else_flag slow_swap, $($flag)*, {
                    "
                        mov rsi, 1 //atomic interrupt
                    "
                } {
                    "
                        mov rsi, 0 //not atomic interrupt
                    "
                }),

                "lea rdx, [rip + {0}]", //main handler
                //stack should be aligned

                "call {1}",

                "pop rax", //gs was swapped
                "cmp rax, 0",
                "je 5f",

                "swapgs",
                "5:",

                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rbp",
                "pop rdi",
                "pop rsi",
                "pop rdx",
                "pop rcx",
                "pop rbx",
                "pop rax",

                //remove err code from stack
                "add rsp, 8",

                "iretq",
                sym $name,
                sym general_interrupt_handler,
            )
        }
        wrapper
    }};

    (@if_flag $target:ident, $($flag:ident)* { $($true:tt)* }) => {
        handler!(@match_flag $target, $($flag)*, {
            $($true)*
        } {""})
    };

    (@if_not_flag $target:ident, $($flag:ident)* { $($true:tt)* }) => {
        handler!(@match_flag $target, $($flag)*, {""} {
            $($true)*
        })
    };

    (@if_else_flag $target:ident, $($flag:ident)*, { $($true:tt)* } { $($false:tt)* }) => {
        handler!(@match_flag $target, $($flag)*, {
            $($true)*
        } {
            $($false)*
        })
    };

    (@match_flag $target:ident, $head:ident $($rest:ident)*, { $($true:tt)* } { $($false:tt)* }) => {
        handler!(@flag_check $target $head { $($true)* } {
            handler!(@match_flag $target, $($rest)*, {
                $($true)*
            } {
                $($false)*
            })
        })
    };

    (@match_flag $target:ident,, { $($true:tt)* } { $($false:tt)* }) => {
        $($false)*
    };


    (@flag_check slow_swap slow_swap { $($true:tt)* } { $($false:tt)* }) => {
        $($true)*
    };
    (@flag_check has_code has_code { $($true:tt)* } { $($false:tt)* }) => {
        $($true)*
    };

    (@flag_check $target:ident $head:ident { $($true:tt)* } { $($false:tt)* }) => {
        $($false)*
    };
}

pub extern "C" fn general_interrupt_handler(
    proc_data: &mut InterruptProcessorState,                   //rdi
    atomic_int: u64,                                           //rsi
    main_handler: extern "C" fn(&mut InterruptProcessorState), //rdx
) {
    let locals = crate::acpi::cpu_locals::CpuLocals::get();
    let prev_atomic = locals.atomic_context;
    locals.int_depth += 1;
    locals.atomic_context |= atomic_int != 0;

    if !locals.atomic_context {
        enable_interrupts();
    }

    main_handler(proc_data);

    //proc is depth 0, root int is depth 1
    if locals.int_depth > 1 || locals.atomic_context {
        disable_interrupts();
        locals.int_depth -= 1;
        locals.atomic_context = prev_atomic;
        return;
    }
    interrupt_context_switch(proc_data);

    //did not context switch -> PROC not initialized or some other "error"
    disable_interrupts();
    locals.int_depth -= 1;
    locals.atomic_context = prev_atomic;
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct InterruptProcessorState {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub err_code: u64,
    pub interrupt_frame: InterruptFrame,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl InterruptProcessorState {
    pub fn new(rip: u64, rsp: u64) -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbp: 0,
            rdi: 0,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            err_code: 0,
            interrupt_frame: InterruptFrame {
                rip,
                cs: 0x23,
                rflags: 0x202,
                rsp,
                ss: 0x1B,
            },
        }
    }
}
