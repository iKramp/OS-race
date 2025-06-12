use super::context_switch::no_ret_context_switch;
use crate::{msr, proc::syscall};
use std::mem_utils::VirtAddr;

mod handlers;

const MSR_STAR: u32 = 0xC000_0081;
const MSR_LSTAR: u32 = 0xC000_0082;
const MSR_CSTAR: u32 = 0xC000_0083;
const MSR_SFMASK: u32 = 0xC000_0084;
const MSR_EFER: u32 = 0xC000_0080;

///Prepare all necessary things for executing syscalls. This includes setting interrupt handlers,
///MSRs and more
pub(super) fn init() {
    let syscall_cs_ss: u16 = 0x8;
    let sysret_cs_ss: u16 = 0x10 | 0x3;
    let syscall_eip: u64 = 0; //unused
    let syscall_rip: u64 = handler_wrapper as *const fn() as u64;
    let compat_rip: u64 = 0; //unused
    let syscall_flag_mask: u32 = 0x700;

    let mut star_reg = (sysret_cs_ss as u64) << 48;
    star_reg |= (syscall_cs_ss as u64) << 32;
    star_reg |= syscall_eip;

    msr::set_msr(MSR_STAR, star_reg);
    msr::set_msr(MSR_LSTAR, syscall_rip);
    msr::set_msr(MSR_CSTAR, compat_rip);
    msr::set_msr(MSR_SFMASK, syscall_flag_mask as u64);
    enable_syscall();
}

fn enable_syscall() {
    let mut efer = msr::get_msr(MSR_EFER);
    efer |= 1;
    msr::set_msr(MSR_EFER, efer);
}

//sys V abi:
//ret val: rax, rdx
//parameters: rdi, rsi, rdx, rcx, r8, r9
//scratch regs: rax, rdi, rsi, rdx, rcx, r8, r9, r10, r11
//preserved: rbx, rsp, rbp, r12 - r15

//syscalls are limited to 5 64bit parameters. If more data is needed, set up a structure and pass a
//pointer to it
#[naked]
extern "C" fn handler_wrapper() -> ! {
    //INFO: any kind of change here should be matched with the one in dispatcher.rs
    unsafe {
        core::arch::naked_asm!(
            //push preserved regs, get kernel stack from gsbase, set current rsp to rax, switch
            //stack is aligned to 16 here
            "sub rsp, 8*8",
            "mov [rsp + 8*7], rcx", //return rip
            "mov [rsp + 8*6], rbx",
            "mov [rsp + 8*5], rbp",
            "mov [rsp + 8*4], r12",
            "mov [rsp + 8*3], r13",
            "mov [rsp + 8*2], r14",
            "mov [rsp + 8*1], r15",
            "mov [rsp + 8*0], r11", //rflags is in r11
            "mov r9, rsp",

            "swapgs",
            "mov cx, 0",
            "mov ss, cx",

            "mov rcx, gs:0", //kernel stack address
            "mov rsp, [rcx]",
            "sti",


            "call {}",
            sym handler
        )
    }
}

#[allow(unused_variables)]
extern "C" fn handler(arg1: u64, arg2: u64, arg3: u64, _return_rcx: u64, arg4: u64, old_rsp: VirtAddr) -> ! {
    //handle here
    // println!("Syscall called with args: {}, {}, {}, {}", arg1, arg2, arg3, arg4);

    #[allow(clippy::single_match)]
    match arg1 {
        1 => syscall::handlers::console_write(arg2, 0, 0),
        _ => {}
    }

    no_ret_context_switch(super::StackCpuStateData::Syscall(SyscallCpuState { rsp: old_rsp }));
}

#[derive(Debug, Clone)]
pub struct SyscallCpuState {
    pub rsp: VirtAddr,
}
