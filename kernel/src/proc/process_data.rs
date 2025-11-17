use std::{boxed::Box, collections::btree_map::BTreeMap, sync::{arc::Arc, no_int_spinlock::NoIntSpinlock}};

use crate::{interrupts::InterruptProcessorState, memory::paging::PageTree, vfs::file::FileHandle};

use super::{syscall::SyscallCpuState, MemoryContext, Pid};


///Describes the process metadata like memory mapping, open files, etc.
#[derive(Debug)]
pub struct ProcessData {
    pid: Pid,
    is_32_bit: bool,
    cmdline: Box<str>,
    internal: NoIntSpinlock<ProcessDataInternal>,
    memory_context: Arc<MemoryContext>,
}

#[derive(Debug)]
struct ProcessDataInternal {
    cpu_state: CpuStateType,
    file_handles: BTreeMap<u64, FileHandle>,
    file_handle_index: u64,
}

#[derive(Debug)]
pub enum CpuStateType {
    Interrupt(InterruptProcessorState),
    Syscall((SyscallCpuState, u64)), //cpu state + userspace stack pointer
    None, //is currently running, was taken
}

pub enum StackCpuStateData<'a> {
    Interrupt(&'a InterruptProcessorState),
    Syscall(&'a SyscallCpuState),
}

impl ProcessData {
    pub fn new(
        pid: Pid,
        is_32_bit: bool,
        cmdline: Box<str>,
        memory_context: Arc<MemoryContext>,
        cpu_state: CpuStateType,
    ) -> Self {
        Self {
            pid,
            is_32_bit,
            cmdline,
            memory_context,
            internal: NoIntSpinlock::new(ProcessDataInternal {
                cpu_state,
                file_handles: BTreeMap::new(),
                file_handle_index: 0,
            }),
        }
    }

    pub fn open_file_handle(&self, handle: FileHandle) -> u64 {
        let internal = &mut self.internal.lock();
        let index = internal.file_handle_index;
        internal.file_handles.insert(index, handle);
        internal.file_handle_index += 1;
        index
    }

    pub fn set_syscall_return(&self, val: u64, err: u64) -> Result<(), ()> {
        let internal = &mut self.internal.lock();
        if let CpuStateType::Syscall((syscall_state, _)) = &mut internal.cpu_state {
            syscall_state.rax = val;
            syscall_state.rdx = err;
            Ok(())
        }
        else {
            Err(())
        }
    }

    pub fn set_cpu_data(&self, cpu_state: CpuStateType) {
        let internal = &mut self.internal.lock();
        internal.cpu_state = cpu_state;
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    pub fn page_tree(&self) -> &PageTree {
        &self.memory_context.get().page_tree
    }

    pub fn take_cpu_state(&self) -> CpuStateType {
        let internal = &mut self.internal.lock();
        core::mem::replace(&mut internal.cpu_state, CpuStateType::None)
    }
}
