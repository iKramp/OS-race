use super::{Pid, Tid};

mod simple_scheduler;
pub(super) use simple_scheduler::SimpleScheduler;

pub(super) trait Scheduler {
    fn accept_new_thread(&mut self, pid: Pid, tid: Tid);
    fn schedule(&mut self) -> Option<(Pid, Tid)>;
    fn remove_process(&mut self, pid: Pid);
    fn remove_thread(&mut self, pid: Pid, tid: Tid);
}
