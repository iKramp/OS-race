use super::ProcessData;


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
pub(super) fn dispatch(proc: &ProcessData) {
    //some kind of assert to make sure we really are the first kernel code from a user process (eg
    //not from an interrupt from existing kernel code)
    //TODO:
    

    todo!("dispatch process {:?}", proc.pid);
}
