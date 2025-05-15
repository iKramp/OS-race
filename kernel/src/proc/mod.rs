use context::{builder::build_mem_context_for_new_proc, info::ContextInfo};
use core::{mem::MaybeUninit, sync::atomic::AtomicU32, time::Duration};
use dispatcher::dispatch;
use scheduler::{Scheduler, SimpleScheduler};
use std::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    mem_utils::VirtAddr,
    rc::Rc,
    string::ToString,
    sync::mutex::{Mutex, MutexGuard},
    thread,
    vec::Vec,
};

use crate::memory::paging::PageTree;

mod context;
mod dispatcher;
mod scheduler;

///stores process metadata
static PROCESSES: Mutex<BTreeMap<Pid, ProcessData>> = Mutex::new(BTreeMap::new());

static SCHEDULER: Mutex<MaybeUninit<Box<dyn Scheduler + Send>>> = Mutex::new(MaybeUninit::uninit());

static PROCESS_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

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
    memory_context: Rc<MemoryContext>,
    state: ProcessState,
}

/// notes:
/// page tree root should always be unique
/// stack size pages should not be larger than [`context::info::MAX_PROC_STACK_SIZE_PAGES`]
struct MemoryContext {
    is_32_bit: bool,
    page_tree: PageTree,
    default_stack_size_pages: u8,
    stacks: Vec<Stack>,
    //shared regions here?
}

/// Describes a stack. The stack is allocated at the top of the address space.
/// stack_base is the highest address of the stack, as it grows down.
/// Stack (in memory) also has mapped memory at the lower edge, non-writeable,
/// non-user-acessible, just so a stack overflow can be detected.
struct Stack {
    stack_base: VirtAddr,
    size_pages: u8,
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
        drop(scheduler_lock);
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
    let process_data = unsafe { &*(process_data as *const ProcessData) };
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

pub fn create_process(context_info: ContextInfo) -> Pid {
    let pid = Pid(PROCESS_ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed));
    let is_32_bit = context_info.is_32_bit();
    let cmdline = context_info.cmdline().to_string().into_boxed_str();
    let memory_context = build_mem_context_for_new_proc(context_info);
    let process_data = ProcessData {
        pid,
        is_32_bit,
        cmdline,
        memory_context: Rc::new(memory_context),
        state: ProcessState::Paused,
    };
    let mut proc_state_lock = PROCESSES.lock();
    proc_state_lock.insert(pid, process_data);
    drop(proc_state_lock);
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    scheduler.accept_new_process(pid);
    panic!("Prepare environment (the fake stack that will be popped by iretq)");
    pid
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
