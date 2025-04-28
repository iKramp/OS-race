use core::{mem::MaybeUninit, ptr::addr_of_mut, time::Duration};
use std::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    mem_utils::{PhysAddr, VirtAddr},
    sync::mutex::{Mutex, MutexGuard},
    thread,
    vec::Vec,
};

use dispatcher::dispatch;
use scheduler::{Scheduler, SimpleScheduler};

mod context;
mod dispatcher;
mod scheduler;

///stores the number of processes using each root page table. Kind of like a custom RC
static ADDRESS_SPACES: Mutex<BTreeMap<PhysAddr, u32>> = Mutex::new(BTreeMap::new());
///stores process metadata
static PROCESSES: Mutex<BTreeMap<Pid, ProcessData>> = Mutex::new(BTreeMap::new());

static SCHEDULER: Mutex<MaybeUninit<Box<dyn Scheduler + Send>>> = Mutex::new(MaybeUninit::uninit());

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Pid(u32);

#[derive(Clone, Copy, Debug)]
enum ProcessState {
    Running,
    Stopping,
    Paused,
}

///Describes the process metadata like memory mapping, open files, etc. Does not describe CPU
///state, as that is stored on the stack when the process is not awake
struct ProcessData {
    pid: Pid,
    is_32_bit: bool,
    cmdline: Box<str>,
    page_tree_root: PhysAddr,
    state: ProcessState,
}

pub fn init() {
    // Initialize the scheduler
    let mut scheduler = SCHEDULER.lock();
    *scheduler = MaybeUninit::new(Box::new(SimpleScheduler::new()));
}

pub fn context_switch() {
    // Switch to the next process
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    let Some(pid) = scheduler.schedule() else {
        //there is no process to run
        thread::sleep(Duration::from_millis(10));
        return;
    };

    let mut process_states_lock = PROCESSES.lock();

    let Some(process_data) = prepare_process_for_run(pid, scheduler_lock, &mut process_states_lock) else {
        //thread is not ready to run
        return;
    };

    //Reference is safe to clone as at most what any other cores can do at this point (after
    //process states lock is dropped) is switch from running to stopping, but that is handled after
    //this execution cycle
    let prcess_data = unsafe { & *(process_data as *const ProcessData)};
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

    if let ProcessState::Stopping = process.state {
        return None;
    }
    drop(scheduler_lock);
    process.state = ProcessState::Running;
    Some(process)
}

//for now this only marks the process as stopping. If it was in running state before, return,
//otherwise clear resources
//Also return if it was in stopping state. Reason: stopping means it's either running and has been
//scheduled for stopping (case above), or its resources are actively being freed
pub fn kill_process(pid: Pid) {
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    scheduler.remove_process(pid);
    let mut proc_state_lock = PROCESSES.lock();
    if let Some(process) = proc_state_lock.get_mut(&pid) {
        let previous_state = process.state;
        process.state = ProcessState::Stopping;
        if !matches!(previous_state, ProcessState::Paused) {
            return;
        }
    }
    drop(scheduler_lock);
    let _process_state = proc_state_lock.remove(&pid);
    drop(proc_state_lock);
    //call a function to clear with process_state
}
