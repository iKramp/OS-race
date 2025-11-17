use crate::{acpi::cpu_locals::CpuLocals, interrupts::InterruptProcessorState, proc::Pid};
use std::{
    sync::arc::Arc, collections::{btree_map::BTreeMap, btree_set::BTreeSet}, vec::Vec
};

use super::{process_data::{self, CpuStateType, StackCpuStateData}, syscall::SyscallCpuState, ProcessData};

pub enum SleepCondition {
    Time(u64),
    ///event will have to wake the process by itself
    Event,
}

pub struct Scheduler {
    tasks: BTreeMap<Pid, Arc<ProcessData>>,
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
        self.tasks.insert(pid, Arc::new(proc_data));
        self.ready_to_run.push(pid);
    }

    pub fn schedule(&mut self) -> Option<Arc<ProcessData>> {
        if self.ready_to_run.is_empty() {
            Self::set_generic_mem_tree();
            return None;
        }
        let pid = self.ready_to_run.remove(0);
        if let Some(proc_data) = self.tasks.get_mut(&pid) {
            let locals = CpuLocals::get();
            // Move the process to active tasks
            self.active_tasks.push((pid, locals.apic_id.into()));
            locals.current_process = Some(proc_data.clone());
            Some(proc_data.clone())
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
        let locals = CpuLocals::get();
        locals.current_process = None;
    }

    fn save_current_proc(&mut self, old_proc: &Arc<ProcessData>, on_stack_data: &StackCpuStateData) {
        let old_proc = &mut old_proc.get();
        match on_stack_data {
            StackCpuStateData::Interrupt(interrupt_frame) => Self::save_interrupted(old_proc, interrupt_frame),
            StackCpuStateData::Syscall(syscall_data) => Self::save_syscalled(old_proc, syscall_data),
        }
    }

    fn save_interrupted(old_proc: &ProcessData, interrupt_frame: &InterruptProcessorState) {
        old_proc.set_cpu_data(CpuStateType::Interrupt(interrupt_frame.clone()));
    }

    fn save_syscalled(old_proc: &ProcessData, syscall_data: &SyscallCpuState) {
        old_proc.set_cpu_data(CpuStateType::Syscall((syscall_data.clone(), CpuLocals::get().userspace_stack_base)));
    }

    ///this function is preferred to avoid locking the mutex twice
    pub fn release_and_schedule(
        &mut self,
        curr_proc: &Option<Arc<ProcessData>>,
        sleep: Option<SleepCondition>,
        on_stack_data: &StackCpuStateData,
    ) -> Option<Arc<ProcessData>> {
        if let Some(proc_data) = curr_proc {
            self.save_current_proc(&proc_data, on_stack_data);
            self.release_process(proc_data.get().pid(), sleep);
        }
        self.schedule()
    }

    pub fn get_proc(&mut self, pid: Pid) -> Option<Arc<ProcessData>> {
        self.tasks.get_mut(&pid).cloned()
    }
}
