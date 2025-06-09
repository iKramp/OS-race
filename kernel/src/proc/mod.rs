use context::{
    builder::create_process,
    info::{ContextInfo, MemoryRegionDescriptor, MemoryRegionFlags},
};
use core::{mem::MaybeUninit, sync::atomic::AtomicU32};
use scheduler::{Scheduler, SimpleScheduler};
use std::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    mem_utils::VirtAddr,
    println,
    string::ToString,
    sync::{arc::Arc, mutex::Mutex},
    vec::Vec,
};
use syscall::SyscallCpuState;

use crate::{interrupts::InterruptProcessorState, memory::paging::PageTree};

mod context;
mod context_switch;
mod dispatcher;
mod loaders;
mod scheduler;
mod syscall;
pub use context_switch::{context_switch, interrupt_context_switch};

///stores process metadata
static PROCESSES: Mutex<BTreeMap<Pid, ProcessData>> = Mutex::new(BTreeMap::new());

static SCHEDULER: Mutex<MaybeUninit<Box<dyn Scheduler + Send>>> = Mutex::new(MaybeUninit::uninit());

static PROCESS_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

static mut PROC_INITIALIZED: bool = false;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Pid(u32);

#[derive(Clone, Copy, Debug)]
enum ProcessState {
    Running,
    Stopping,
    Paused,
}

///Describes the process metadata like memory mapping, open files, etc.
#[derive(Debug)]
struct ProcessData {
    pid: Pid,
    is_32_bit: bool,
    cmdline: Box<str>,
    memory_context: Arc<MemoryContext>,
    proc_state: ProcessState,
    cpu_state: CpuStateType,
}

#[derive(Debug)]
enum CpuStateType {
    Interrupt(InterruptProcessorState),
    Syscall(SyscallCpuState),
}

pub enum StackCpuStateData<'a> {
    Interrupt(&'a InterruptProcessorState),
    Syscall(SyscallCpuState), //nothing on kernel stack
}

/// notes:
/// page tree root should always be unique
/// stack size pages should not be larger than [`context::info::MAX_PROC_STACK_SIZE_PAGES`]
#[derive(Debug)]
struct MemoryContext {
    is_32_bit: bool,
    page_tree: PageTree,
    stacks: Vec<Stack>,
    //shared regions here?
}

/// Describes a stack. The stack is allocated at the top of the address space.
/// stack_base is the highest address of the stack, as it grows down.
/// Stack (in memory) also has mapped memory at the lower edge, non-writeable,
/// non-user-acessible, just so a stack overflow can be detected.
#[derive(Debug)]
struct Stack {
    stack_base: VirtAddr,
    size_pages: u8,
}

pub fn print_context_mem_trees() {
    let proc_lock = PROCESSES.lock();
    for (_, process) in proc_lock.iter() {
        process.memory_context.get().page_tree.print_mapping();
    }
}

pub fn init() {
    // Initialize the scheduler
    let mut scheduler = SCHEDULER.lock();
    *scheduler = MaybeUninit::new(Box::new(SimpleScheduler::new()));
    drop(scheduler);
    create_fallback_process();
    loaders::init_process_loaders();

    let context_info = loaders::load_process(crate::TEST_EXECUTABLE).expect("Failed to load test executable");
    let pid = create_process(context_info);
    println!("Created process with pid: {:?}", pid);

    syscall::init();
}

//set this AFTER the process with pid 1 is loaded (pid 0 is fallback, might be removed)
pub fn set_proc_initialized() {
    unsafe {
        PROC_INITIALIZED = true;
    }
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
        let previous_state = process.proc_state;
        process.proc_state = ProcessState::Stopping;
        if !matches!(previous_state, ProcessState::Paused) {
            return;
        }
    }
    drop(scheduler_lock);
    let _proc_data = proc_state_lock.remove(&pid);
    drop(proc_state_lock);
    //call a function to clear with process_state
}

//context switch to this process when no other processes exist
pub fn create_fallback_process() {
    let code_region = MemoryRegionDescriptor::new(VirtAddr(0x1000), 1, MemoryRegionFlags(2)).unwrap();
    let data_region = MemoryRegionDescriptor::new(VirtAddr(0x2000), 1, MemoryRegionFlags(1)).unwrap();
    let code_init = [
        0x90,                  //nop
        0x90,                  //nop
        0x90,                  //nop
        0x90,                  //nop
        0x48,                  //vvv
        0xC7,                  //vvv
        0xC7,                  //mov rdi, imm
        0x01,                  //vvv
        0x00,                  //vvv
        0x00,                  //vvv
        0x00,                  //0x1
        0x48,                  //vvv
        0xC7,                  //vvv
        0xC6,                  //mov rsi, imm
        0x00,                  //vvv
        0x20,                  //vvv
        0x00,                  //vvv
        0x00,                  //0x2000
        0x0f,                  //vvv
        0x05,                  //syscall
        0x90,                  //nop
        0xEB,                  //jmp
        0_u8.wrapping_sub(20), //jmp offset
    ];
    let data_init = b"Message from user process: uhhhh idk something something works??\0";

    let fake_context = ContextInfo::new(
        false,
        &mut [code_region, data_region],
        Box::new([(VirtAddr(0x1000), &code_init), (VirtAddr(0x2000), data_init)]),
        VirtAddr(0x1000),
        "fallback_process".to_string().into_boxed_str(),
    )
    .unwrap();
    let pid = create_process(fake_context);
    assert_eq!(pid.0, 0);
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    scheduler.remove_process(pid);
}
