use std::time::{GET_TIME, UNIX_EPOCH};

use crate::proc::syscall::SyscallArgs;


pub fn time(args: &SyscallArgs) -> bool {
    let time = unsafe { GET_TIME() };
    let duration = time.duration_since(UNIX_EPOCH);
    unsafe {
        *(args.arg1 as *mut u64) = duration.as_secs();
        *(args.arg2 as *mut u64) = duration.subsec_nanos() as u64;
    }
    false
}
