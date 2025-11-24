use std::sync::arc::Arc;

use crate::proc::{syscall::SyscallArgs, ProcessData};


pub fn fclose(args: &mut SyscallArgs, proc: &Arc<ProcessData>) -> bool {
    let fd = args.arg1;
    let mut proc_mut = proc.get_mutable();
    if proc_mut.take_file_handle(fd).is_some() {
        proc.set_syscall_return(0, 0).unwrap();
    } else {
        proc.set_syscall_return(u64::MAX, 1).unwrap();
    }
    false
}
