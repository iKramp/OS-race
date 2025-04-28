use super::ThreadData;

/*
 * Things that need to be done: (Intel SDM, Vol 3, chapter 8.1.2
 * Keep segment registers CS, DS, SS, ES, FS, Gs the same (do nothing)
 * Push general purpose registers. After this, they can be modified again to aid in saving the rest
 * oof the state
 * Push E/RFLAGS
 * Push RIP
 * Push CR3
 * Update CPU locals to indicate a process being run?
 * Save fpu, mmx... state with fxsave64. Enable REX.W
 * save/restore gs and fs registers  through MSRs and swapgs
 */
 
//this function should NOT use the heap at all to prevent memory leaks by setting IP and SP
pub(super) fn dispatch(thread: ThreadData) {
    todo!("dispatch process {:?}, thread {:?}", thread.pid, thread.thread_id);
}
