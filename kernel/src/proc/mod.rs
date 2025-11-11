use context::{
    builder::create_process,
    info::{ContextInfo, MemoryRegionDescriptor, MemoryRegionFlags},
};
use core::{mem::MaybeUninit, sync::atomic::AtomicU32};
use scheduler::Scheduler;
use std::{
    boxed::Box,
    mem_utils::VirtAddr,
    println,
    string::ToString,
    sync::{arc::Arc, no_int_spinlock::NoIntSpinlock},
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

static SCHEDULER: NoIntSpinlock<MaybeUninit<Scheduler>> = NoIntSpinlock::new(MaybeUninit::uninit());

static PROCESS_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

pub static mut PROC_INITIALIZED: bool = false;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Pid(pub u32);

///Describes the process metadata like memory mapping, open files, etc.
#[derive(Debug)]
struct ProcessData {
    pid: Pid,
    is_32_bit: bool,
    cmdline: Box<str>,
    memory_context: Arc<MemoryContext>,
    cpu_state: CpuStateType,
}

#[derive(Debug)]
enum CpuStateType {
    Interrupt(InterruptProcessorState),
    Syscall((SyscallCpuState, u64)), //cpu state + userspace stack pointer
}

pub enum StackCpuStateData<'a> {
    Interrupt(&'a InterruptProcessorState),
    Syscall(&'a SyscallCpuState),
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

pub fn init() {
    // Initialize the scheduler
    let mut scheduler = SCHEDULER.lock();
    *scheduler = MaybeUninit::new(Scheduler::new());
    drop(scheduler);
    create_fallback_process();
    loaders::init_process_loaders();

    let prime_finder = loaders::load_process(crate::PRIME_FINDER).expect("Failed to load test executable prime finders");
    for _i in 0..10 {
        let pid = create_process(&prime_finder);
        println!("Created process with pid: {:?}", pid);
    }

    let time_printer = loaders::load_process(crate::TIME_PRINTER).expect("Failed to load test executable time printer");
    for _i in 0..10 {
        let pid = create_process(&time_printer);
        println!("Created process with pid: {:?}", pid);
    }

    syscall::init();
    set_proc_initialized();
}

pub fn init_ap() {
    syscall::init();
}

//set this AFTER the process with pid 1 is loaded (pid 0 is fallback, might be removed)
pub fn set_proc_initialized() {
    unsafe {
        PROC_INITIALIZED = true;
    }
    let locals = crate::acpi::cpu_locals::CpuLocals::get();
    locals.proc_initialized = true;
}

//for now this only marks the process as stopping. If it was in running state before, return,
//otherwise clear resources
//Also return if it was in stopping state. Reason: stopping means it's either running and has been
//scheduled for stopping (case above), or its resources are actively being freed
pub fn kill_process(pid: Pid) {
    unsafe { SCHEDULER.lock().assume_init_mut().remove_process(pid) };
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
    let pid = create_process(&fake_context);
    assert_eq!(pid.0, 0);
    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    scheduler.remove_process(pid);
}
