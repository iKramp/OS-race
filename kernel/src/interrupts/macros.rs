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

                    // handler!(@if_flag needs_code, $($flag)* => {
                    //     "push 0"
                    // }),

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

                    //mov pushed cs to register by offsetting from rsp
                    "xor rax, rax",
                    "mov rax, [rsp + 8 * 17] //cs",

                    handler!(@if_else_flag rax, $($flag)* => {
                        //check if we need to swap gs_base
                        "je 2b"
                    }, else => {
                        //skip swapgs
                        "jmp 2b",
                    }),

                    //skip swapgs
                    "2:",
                    "jmp 2b",

                    //check for swapping gs_base

                    "mov rdi, rsp",
                    "call {}",

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

                    "iretq",
                    sym $name,
                )

            }
        }
        wrapper
    }};

    // (@push_err_code true) => {
    //     ""
    // };
    //
    // (@push_err_code false) => {
    //     "push 0"
    // };

    (@if_flag $target:ident, $($flag:ident)* => { $($true:tt)* }) => {
        handler!(@match_flag $target, $($flag)* => {
            $($true)*
        }, else => {})
    };

    (@if_else_flag $target:ident, $($flag:ident)* => { $($true:tt)* }, else => { $($false:tt)* }) => {
        handler!(@match_flag $target, $($flag)* => {
            $($true)*
        }, else => {
            $($false)*
        })
    };

    (@match_flag $target:ident, $head:ident $($rest:ident)* => { $($true:tt)* }, else => { $($false:tt)* }) => {
        static_cond!(if $target == $head $($true)* else {
            handler!(@match_flag $target, $($rest)* => {
                $($true)*
            }, else => {
                $($false)*
            })
        })
    };

    // (@is_same $a:ident, $b:ident => { $($true:tt)* }, else => { $($false:tt)* }) => {
    //     macro_rules! __inner {
    //         ($a $a) => { $($true)* };
    //         ($a $b) => { $($false)* };
    //     }
    //
    //     __inner!($a $b)
    // };
}

//write types that reference the stack


macro_rules! static_cond {
    // private rule to define and call the local macro
    // we evaluate a conditional by generating a new macro (in an inner scope, so name shadowing is
    // not a big concern) and calling it
    (@expr $lhs:tt $rhs:tt $($arm1:tt)* else $($arm2:tt)*) => {{
        // note that the inner macro has no captures (it can't, because there's no way to escape `$`)
        macro_rules! __static_cond {
            ($lhs $lhs) => $arm1;
            ($lhs $rhs) => $arm2;
        }

        __static_cond!($lhs $rhs)
    }};

    // no else condition provided: fall through with empty else
    (if $lhs:tt == $rhs:tt $($then:tt)*) => {
        $crate::static_cond!(if $lhs == $rhs $($then)* else "",)
    };
    (if $lhs:tt != $rhs:tt $($then:tt)*) => {
        $crate::static_cond!(if $lhs != $rhs $($then)* else "",)
    };

    // main entry point with then and else arms
    (if $lhs:tt == $rhs:tt $($then:tt)* else $($els:tt)*) => {
        $crate::static_cond!(@expr $lhs $rhs $($then)* else $($els)*)
	};
    (if $lhs:tt != $rhs:tt $($then:tt)* else $($els:tt)*) => {
        $crate::static_cond!(@expr $lhs $rhs $($els)* else $($then)*)
    };
}
