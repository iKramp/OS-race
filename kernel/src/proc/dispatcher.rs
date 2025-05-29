use crate::{interrupts::ProcessorState, memory::paging};

use super::{ProcessData, ProcessState};

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
pub(super) fn dispatch(new_proc: &ProcessData, old_proc: Option<&mut ProcessData>, return_frame: &mut ProcessorState) {
    let new_page_tree = &new_proc.memory_context.get().page_tree;
    paging::PageTree::set_level4_addr(new_page_tree.root());
    if let Some(old_proc) = old_proc {
        // Save the current process state
        old_proc.cpu_state = return_frame.clone();
        old_proc.proc_state = ProcessState::Paused;
    }
    *return_frame = new_proc.cpu_state.clone();
}

pub(super) fn is_root_interrupt(return_frame: &mut ProcessorState) -> bool {
    return_frame.cs == 0x16
}
