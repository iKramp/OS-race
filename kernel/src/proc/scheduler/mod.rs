use super::Pid;

mod simple_scheduler;
pub(super) use simple_scheduler::SimpleScheduler;

pub(super) trait Scheduler {
    fn accept_new_thread(&mut self, pid: Pid);
    fn schedule(&mut self) -> Option<Pid>;
    fn remove_process(&mut self, pid: Pid);
    fn remove_thread(&mut self, pid: Pid);
}
