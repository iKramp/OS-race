use core::ffi::c_char;
use std::{format, println, string::ToString};

use crate::{acpi::cpu_locals, proc::syscall::SyscallArgs};

pub extern "C" fn console_write(args: &SyscallArgs) -> bool {
    let str_ptr = unsafe { core::ffi::c_str::CStr::from_ptr(args.arg1 as *const c_char) };
    let rust_str = str_ptr.to_str();
    let cpu_locals = cpu_locals::CpuLocals::get();
    let current_pid = cpu_locals.current_process.as_ref().map_or(0, |p| p.get().pid().0);
    let pid_str = if current_pid != 0 {
        format!("proc {}", current_pid)
    } else {
        "no process???".to_string()
    };
    let info_str = format!("[CPU {}, proc {}]", cpu_locals.processor_id, pid_str);
    if let Ok(rust_str) = rust_str {
        println!("{}: {}", info_str, rust_str);
    }
    false
}
