#[macro_export]
macro_rules! handler {
    (
        $name:ident $(, $flag:ident )* $(,)?
    ) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
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
                            3:
                            push rdx //save for later
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
                            3:
                            push rax //save for later
                        "
                    }),

                    "mov rdi, rsp",
                    "add rdi, 8", //start of proc data
                    
                    "sti", //enable interrupts (nesting)

                    //stack should be aligned

                    "call {}",

                    "cli", //disable interrupts

                    handler!(@if_else_flag slow_swap, $($flag)*, {
                        "
                            pop rdx //restore gsbase
                            test edx, edx
                            js 4f

                            swapgs
                            4:
                        "
                    } {
                        "
                            pop rax
                            cmp rax, 8
                            je 4f

                            swapgs
                            4:
                        "
                    }),

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
                )

            }
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

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ProcessorState {
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
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl ProcessorState {
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
            rip,
            cs: 0x1B,
            rflags: 0x3202,
            rsp,
            ss: 0x23,
        }
    }
}
