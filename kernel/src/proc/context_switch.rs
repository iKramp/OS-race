use crate::{acpi::cpu_locals::CpuLocals, interrupts::InterruptProcessorState};

use super::{PROC_INITIALIZED, ProcessData, SCHEDULER, dispatcher::dispatch, process_data::StackCpuStateData};

pub extern "C" fn interrupt_context_switch(on_stack_data: &mut InterruptProcessorState) {
    context_switch(StackCpuStateData::Interrupt(on_stack_data));
}

pub fn context_switch(on_stack_data: StackCpuStateData) {
    let locals = CpuLocals::get();
    if !unsafe { PROC_INITIALIZED } || !locals.proc_initialized {
        return;
    }

    #[cfg(debug_assertions)] //interrupts should already check this
    if locals.int_depth != 1 || locals.atomic_context {
        panic!(
            "Invalid context switch state: int_depth = {}, atomic_context = {}",
            locals.int_depth, locals.atomic_context
        );
    }

    no_ret_context_switch(on_stack_data);
}

pub fn no_ret_context_switch(on_stack_data: StackCpuStateData) -> ! {
    let cpu_locals = CpuLocals::get();
    let current_pid = &cpu_locals.current_process;

    // Switch to the next process
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    let next_proc = scheduler.release_and_schedule(current_pid, None, &on_stack_data);
    if let Some(process_data_arc) = next_proc {
        cpu_locals.current_process = Some(process_data_arc.clone());
        let process_data_ptr = process_data_arc.get() as *const ProcessData;
        drop(process_data_arc);
        let process_data = unsafe { &*process_data_ptr }; //safe because it's saved in cpu locals
        drop(scheduler_lock);
        dispatch(process_data)
    } else {
        drop(scheduler_lock);
        loop {
            //wait here
            std::thread::sleep(core::time::Duration::from_millis(10));

            let mut scheduler_lock = SCHEDULER.lock();
            let scheduler = unsafe { scheduler_lock.assume_init_mut() };
            if let Some(process_data_arc) = scheduler.schedule() {
                cpu_locals.current_process = Some(process_data_arc.clone());
                let process_data_ptr = process_data_arc.get() as *const ProcessData;
                drop(process_data_arc);
                let process_data = unsafe { &*process_data_ptr }; //safe because it's saved in cpu locals
                drop(scheduler_lock);
                dispatch(process_data)
            }
        }
    }
}
