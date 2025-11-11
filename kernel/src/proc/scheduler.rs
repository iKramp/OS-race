use crate::{acpi::cpu_locals::CpuLocals, interrupts::InterruptProcessorState, proc::Pid};
use std::{
    boxed::Box,
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    vec::Vec,
};

use super::{CpuStateType, ProcessData, StackCpuStateData, syscall::SyscallCpuState};

pub enum SleepCondition {
    Time(u64),
    ///event will have to wake the process by itself
    Event,
}

pub struct Scheduler {
    tasks: BTreeMap<Pid, Box<ProcessData>>,
    sleeping_tasks: Vec<(Pid, SleepCondition)>,
    ///tasks currently running on the CPU, along with cpu number
    active_tasks: Vec<(Pid, u32)>,
    ready_to_run: Vec<Pid>,
    purge_queue: BTreeSet<Pid>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            sleeping_tasks: Vec::new(),
            active_tasks: Vec::new(),
            ready_to_run: Vec::new(),
            purge_queue: BTreeSet::new(),
        }
    }
}

impl Scheduler {
    pub fn accept_new_process(&mut self, pid: Pid, proc_data: ProcessData) {
        self.tasks.insert(pid, Box::new(proc_data));
        self.ready_to_run.push(pid);
    }

    pub fn schedule(&mut self) -> Option<&ProcessData> {
        if self.ready_to_run.is_empty() {
            Self::set_generic_mem_tree();
            return None;
        }
        let pid = self.ready_to_run.remove(0);
        if let Some(proc_data) = self.tasks.get_mut(&pid) {
            // Move the process to active tasks
            self.active_tasks.push((pid, 0)); // Assuming CPU 0 for simplicity
            Some(proc_data)
        } else {
            Self::set_generic_mem_tree();
            None
        }
    }

    fn set_generic_mem_tree() {
        todo!();
    }

    pub fn remove_process(&mut self, pid: Pid) {
        let sleeping_pos = self.sleeping_tasks.iter().position(|(p, _)| *p == pid);
        if let Some(pos) = sleeping_pos {
            self.sleeping_tasks.swap_remove(pos);
        }
        let ready_pos = self.ready_to_run.iter().position(|p| *p == pid);
        if let Some(pos) = ready_pos {
            self.ready_to_run.swap_remove(pos);
        }
        self.purge_queue.insert(pid);
    }

    ///Called after all the data has been saved
    fn release_process(&mut self, pid: Pid, sleep: Option<SleepCondition>) {
        let active_pos = self.active_tasks.iter().position(|(p, _)| *p == pid);
        if let Some(pos) = active_pos {
            self.active_tasks.swap_remove(pos);
        } else {
            //something went seriously wrong. Just in case purge the process. Might crash the pc
            //idk, but this should be unreachable anyway
            self.purge_queue.insert(pid);
        }

        if self.purge_queue.remove(&pid) {
            todo!("implement removing a process");
        } else {
            if let Some(cond) = sleep {
                self.sleeping_tasks.push((pid, cond));
            } else {
                self.ready_to_run.push(pid);
            }
        }
    }

    fn save_current_proc(&mut self, old_proc: Pid, on_stack_data: &StackCpuStateData) {
        let old_proc = self.tasks.get_mut(&old_proc);
        if let Some(old_proc) = old_proc {
            match on_stack_data {
                StackCpuStateData::Interrupt(interrupt_frame) => Self::save_interrupted(old_proc, interrupt_frame),
                StackCpuStateData::Syscall(syscall_data) => Self::save_syscalled(old_proc, syscall_data),
            }
        }
    }

    fn save_interrupted(old_proc: &mut ProcessData, interrupt_frame: &InterruptProcessorState) {
        old_proc.cpu_state = CpuStateType::Interrupt(interrupt_frame.clone());
    }

    fn save_syscalled(old_proc: &mut ProcessData, syscall_data: &SyscallCpuState) {
        old_proc.cpu_state = CpuStateType::Syscall((syscall_data.clone(), CpuLocals::get().userspace_stack_base));
    }

    ///this function is preferred to avoid locking the mutex twice
    pub fn release_and_schedule(
        &mut self,
        pid: Pid,
        sleep: Option<SleepCondition>,
        on_stack_data: &StackCpuStateData,
    ) -> Option<&ProcessData> {
        if pid.0 != 0 {
            self.save_current_proc(pid, on_stack_data);
            self.release_process(pid, sleep);
        }
        self.schedule()
    }
}
