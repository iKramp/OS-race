use std::{boxed::Box, string::ToString};

use crate::{proc::{syscall::SyscallArgs, Pid}, task_runner, vfs::{self, file::FileFlags, InodeIdentifier}};


pub fn fopen(args: &mut SyscallArgs, pid: Pid) {
    let c_path = unsafe { core::ffi::c_str::CStr::from_ptr(args.arg1 as *const i8) };
    let Ok(path) = c_path.to_str() else {
        args.syscall_number = u64::MAX;
        return;
    };
    let path = path.to_string();

    let fd = args.arg2;
    let ftags = args.arg3;
    let _create_mode = args.arg4;

    let file_source: Option<InodeIdentifier> = if fd == 0 {
        None
    } else {
        None //for now
    };

    let task = async move {
        let ret_val = 0;
        let resolved_path = vfs::resolve_path(&path);
        let file_flags = FileFlags(ftags as u8);
        let handle = vfs::open_file((&resolved_path).into(), file_source, file_flags).await;
        match handle {
            Ok(handle) => {
                let Some(proc) = crate::proc::get_proc(pid) else {
                    return; //proc was killed
                };
                
                let proc_lock = proc.get();
                proc_lock.open_file_handle(handle);
                proc_lock.set_syscall_return(ret_val, 0).unwrap();

            },
            Err(_) => {
                let Some(proc) = crate::proc::get_proc(pid) else {
                    return; //proc was killed
                };
                
                let proc_lock = proc.get();
                proc_lock.set_syscall_return(u64::MAX, 1).unwrap();
            }
        }
    };

    task_runner::add_task(Box::pin(task));
}
