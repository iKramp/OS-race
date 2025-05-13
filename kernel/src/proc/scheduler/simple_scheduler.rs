use super::Scheduler;
use crate::proc::Pid;
use std::vec::Vec;

pub struct SimpleScheduler {
    tasks: Vec<Pid>,
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
    fn accept_new_process(&mut self, pid: Pid) {
        // Add the new process to the scheduler
        self.tasks.push(pid);
    }

    fn schedule(&mut self) -> Option<Pid> {
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
        self.tasks.retain(|&p| p != pid);
    }
}
