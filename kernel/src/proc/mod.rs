use core::{mem::MaybeUninit, time::Duration};
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Tid(u32);

#[derive(Clone, Copy, Debug)]
enum ThreadState {
    Running,
    Sleeping,
    Stopping,
}

enum ProcessState {
    Running,
    Stopping,
}

///Describes the process metadata like memory mapping, open files, etc. Does not describe CPU
///state, as that is stored on the stack when the process is not awake
struct ProcessData {
    pid: Pid,
    is_32_bit: bool,
    cmdline: Box<str>,
    page_tree_root: PhysAddr,
    threads: Vec<ThreadData>,
    state: ProcessState,
}

#[derive(Clone, Debug)]
struct ThreadData {
    pid: Pid,
    thread_id: Tid,
    stack_pointer: VirtAddr,
    stack_base: VirtAddr,
    stack_size_pages: u8,
    state: ThreadState,
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
    let Some((pid, tid)) = scheduler.schedule() else {
        //there is no process to run
        thread::sleep(Duration::from_millis(10));
        return;
    };

    let mut process_states_lock = PROCESSES.lock();

    let Some(thread_data) = prepare_thread_for_run(pid, tid, scheduler_lock, &mut process_states_lock) else {
        //thread is not ready to run
        return;
    };
    //any other thread checks come above this line, but this shoutl be the end of it
    let thread_data = thread_data.clone();
    drop(process_states_lock);

    dispatch(thread_data);
}

fn prepare_thread_for_run<'a>(
    pid: Pid,
    tid: Tid,
    mut scheduler_lock: MutexGuard<MaybeUninit<Box<dyn Scheduler + Send>>>,
    proc_state_lock: &'a mut MutexGuard<BTreeMap<Pid, ProcessData>>,
) -> Option<&'a mut ThreadData> {
    let Some(process) = proc_state_lock.get_mut(&pid) else {
        //process not found
        unsafe { scheduler_lock.assume_init_mut().remove_process(pid) };
        return None;
    };

    if let ProcessState::Stopping = process.state {
        return None;
    }

    let Some(thread) = process.threads.iter_mut().find(|t| t.thread_id == tid) else {
        //thread not found
        unsafe { scheduler_lock.assume_init_mut().remove_thread(pid, tid) };
        return None;
    };
    drop(scheduler_lock);
    if let ThreadState::Stopping = thread.state {
        return None;
    }
    thread.state = ThreadState::Running;
    Some(thread)
}

//For now this only marks the process as stopping, but in reality we must also check and if there
//are no threads being run, we must free resources and clear the process
//If threads are still being run, this is enough, and resources will be freed when the last
//thread yields
pub fn kill_thread(pid: Pid, tid: Tid) {
    if pid.0 == tid.0 {
        //thread is the main thread, so we need to kill the whole process
        kill_process(pid);
        return;
    }
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    scheduler.remove_thread(pid, tid);
    let mut proc_state_lock = PROCESSES.lock();
    if let Some(process) = proc_state_lock.get_mut(&pid) {
        if let Some(thread) = process.threads.iter_mut().find(|t| t.thread_id == tid) {
            thread.state = ThreadState::Stopping;
        }
    }
}

pub fn kill_process(pid: Pid) {
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    scheduler.remove_process(pid);
    let mut proc_state_lock = PROCESSES.lock();
    if let Some(process) = proc_state_lock.get_mut(&pid) {
        process.state = ProcessState::Stopping;
    }
}
