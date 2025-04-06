use std::vec::Vec;
use crate::proc::{Pid, Tid};
use super::Scheduler;


pub struct SimpleScheduler {
    tasks: Vec<(Pid, Tid)>,
    current_task: usize,
}

impl SimpleScheduler {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_task: 0,
        }
    }
}

impl Scheduler for SimpleScheduler {
    fn accept_new_thread(&mut self, pid: Pid, tid: Tid) {
        // Add the new process to the scheduler
        self.tasks.push((pid, tid));
    }

    fn schedule(&mut self) -> Option<(Pid, Tid)> {
        if self.tasks.is_empty() {
            return None;
        }
        self.current_task += 1;
        if self.current_task >= self.tasks.len() {
            self.current_task = 0;
        }
        let pid = self.tasks[self.current_task];
        Some(pid)
    }

    fn remove_process(&mut self, pid: crate::proc::Pid) {
        // Remove the process from the scheduler
        self.tasks.retain(|&(p, _)| p != pid);
    }

    fn remove_thread(&mut self, pid: crate::proc::Pid, tid: crate::proc::Tid) {
        // Remove the thread from the scheduler
        self.tasks.retain(|&(p, t)| p != pid || t != tid);
    }
}
