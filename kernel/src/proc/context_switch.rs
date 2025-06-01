use core::mem::MaybeUninit;
use std::{boxed::Box, collections::btree_map::BTreeMap, sync::mutex::MutexGuard};

use crate::{acpi::cpu_locals::CpuLocals, interrupts::InterruptProcessorState};

use super::{
    PROC_INITIALIZED, PROCESSES, Pid, ProcessData, ProcessState, SCHEDULER, StackCpuStateData,
    dispatcher::{dispatch, is_root_interrupt, save_current_proc},
    scheduler::Scheduler,
};

pub extern "C" fn interrupt_context_switch(on_stack_data: &mut InterruptProcessorState) {
    context_switch(StackCpuStateData::Interrupt(on_stack_data), false);
}

pub fn context_switch(on_stack_data: StackCpuStateData, force_switch: bool) {
    if !force_switch && !is_root_interrupt(&on_stack_data) {
        return;
    }

    if !unsafe { PROC_INITIALIZED } {
        return;
    }

    // Switch to the next process
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    let pid = match scheduler.schedule() {
        Some(pid) => pid,
        None => {
            //fallback process with nops. Fallback process should just sleep
            Pid(0)
        }
    };

    let mut process_states_lock = PROCESSES.lock();

    let Some(process_data) = prepare_process_for_run(pid, scheduler_lock, &mut process_states_lock) else {
        //thread is not ready to run
        return;
    };

    //---------commit point---------

    //Reference is safe to clone as at most what any other cores can do at this point (after
    //process states lock is dropped) is switch from running to stopping, but that is handled after
    //this execution cycle
    let process_data = unsafe { &*(process_data as *const ProcessData) };

    let cpu_locals = CpuLocals::get();
    let current_pid = cpu_locals.current_process;
    let current_proc_data = process_states_lock.get_mut(&Pid(current_pid));

    save_current_proc(current_proc_data, on_stack_data);
    drop(process_states_lock);

    dispatch(process_data);
}

fn prepare_process_for_run<'a>(
    pid: Pid,
    mut scheduler_lock: MutexGuard<MaybeUninit<Box<dyn Scheduler + Send>>>,
    proc_state_lock: &'a mut MutexGuard<BTreeMap<Pid, ProcessData>>,
) -> Option<&'a mut ProcessData> {
    let Some(process) = proc_state_lock.get_mut(&pid) else {
        //process not found
        unsafe { scheduler_lock.assume_init_mut().remove_process(pid) };
        return None;
    };

    if let ProcessState::Stopping = process.proc_state {
        return None;
    }
    drop(scheduler_lock);
    process.proc_state = ProcessState::Running;
    Some(process)
}
