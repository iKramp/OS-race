use crate::{interrupts::InterruptProcessorState, memory::paging};

use super::{CpuStateType, ProcessData, StackCpuStateData, syscall::SyscallCpuState};

/*
 * Things that need to be done: (Intel SDM, Vol 3, chapter 8.1.2
 * Keep segment registers CS, DS, SS, ES, FS, Gs the same (do nothing)
 * Push general purpose registers. After this, they can be modified again to aid in saving the rest
 * of the state
 * Push E/RFLAGS
 * Push RIP
 * Push CR3
 * Update CPU locals to indicate a process being run?
 * Save fpu, mmx... state with fxsave64. Enable REX.W
 * save/restore gs and fs registers  through MSRs and swapgs
 */

//this function should NOT use the heap at all to prevent memory leaks by setting IP and SP
pub(super) fn dispatch(new_proc: &ProcessData) -> ! {
    //INFO: any kind of change here should be matched with the one in interrupts/macros.rs and
    //syscall.rs

    let new_page_tree = &new_proc.memory_context.get().page_tree;
    paging::PageTree::set_level4_addr(new_page_tree.root());
    unsafe {
        core::arch::asm!(
            "cli", //disable interrupts
            //this is a bit tricky. We can do this because context switch is only called on
            //syscalls (they did swapgs) and non-async interrupts, and those interrupts can check
            //(and did do so) if they are the root interrupt, meaning they did swapgs
            "swapgs"
        );
    }

    match &new_proc.cpu_state {
        CpuStateType::Interrupt(interrupt_frame) => return_interrupted(interrupt_frame),
        CpuStateType::Syscall(state) => return_syscalled(state),
    }
}

pub(super) fn save_current_proc(old_proc: Option<&mut ProcessData>, on_stack_data: StackCpuStateData) {
    if let Some(old_proc) = old_proc {
        match on_stack_data {
            StackCpuStateData::Interrupt(interrupt_frame) => save_interrupted(old_proc, interrupt_frame),
            StackCpuStateData::Syscall(syscall_data) => save_syscalled(old_proc, &syscall_data),
        }
    }
}

fn save_interrupted(old_proc: &mut ProcessData, interrupt_frame: &InterruptProcessorState) {
    old_proc.cpu_state = CpuStateType::Interrupt(interrupt_frame.clone());
}

fn save_syscalled(old_proc: &mut ProcessData, syscall_data: &SyscallCpuState) {
    old_proc.cpu_state = CpuStateType::Syscall(syscall_data.clone());
}

fn return_interrupted(interrupt_frame: &InterruptProcessorState) -> ! {
    //INFO: any kind of change here should be matched with the one in interrupts/macros.rs

    //make rsp at least return frame size smaller than the start of a page
    let interrupt_frame_addr: u64 = interrupt_frame as *const InterruptProcessorState as u64;
    unsafe {
        core::arch::asm!(
            "mov rsp, {0}",
            "mov r15, [rsp + 8 * 0]",
            "mov r14, [rsp + 8 * 1]",
            "mov r13, [rsp + 8 * 2]",
            "mov r12, [rsp + 8 * 3]",
            "mov r11, [rsp + 8 * 4]",
            "mov r10, [rsp + 8 * 5]",
            "mov r9,  [rsp + 8 * 6]",
            "mov r8,  [rsp + 8 * 7]",
            "mov rbp, [rsp + 8 * 8]",
            "mov rdi, [rsp + 8 * 9]",
            "mov rsi, [rsp + 8 * 10]",
            "mov rdx, [rsp + 8 * 11]",
            "mov rcx, [rsp + 8 * 12]",
            "mov rbx, [rsp + 8 * 13]",
            "mov rax, [rsp + 8 * 14]",
            //rsp + 8 * 15 is error code
            "add rsp, 8 * 16",
            "iretq",

            in(reg) interrupt_frame_addr
        );
    }
    unreachable!();
}

fn return_syscalled(cpu_state: &SyscallCpuState) -> ! {
    //INFO: any kind of change here should be matched with the one in syscall.rs
    unsafe {
        core::arch::asm!(
            "mov rsp, {0}",
            //both segments are restored automatically by sysret
            "mov r11, [rsp + 8*0]", //rflags
            "mov r15, [rsp + 8*1]",
            "mov r14, [rsp + 8*2]",
            "mov r13, [rsp + 8*3]",
            "mov r12, [rsp + 8*4]",
            "mov rbp, [rsp + 8*5]",
            "mov rbx, [rsp + 8*6]",
            "mov rcx, [rsp + 8*7]",

            "add rsp, 8*8",
            "sysretq",

            in(reg) cpu_state.rsp.0,
        )
    };
    unreachable!()
}

pub(super) fn is_root_interrupt(on_stack_data: &StackCpuStateData) -> bool {
    match on_stack_data {
        StackCpuStateData::Interrupt(interrupt_state) => interrupt_state.interrupt_frame.cs == 0x16,
        StackCpuStateData::Syscall(_) => true,
    }
}
