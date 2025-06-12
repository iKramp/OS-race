use core::ffi::c_char;
use std::{format, println};

use crate::acpi::cpu_locals;

pub extern "C" fn console_write(string_ptr: u64, _: u64, _: u64) {
    let str_ptr = unsafe { core::ffi::c_str::CStr::from_ptr(string_ptr as *const c_char) };
    let rust_str = str_ptr.to_str();
    let cpu_locals = cpu_locals::CpuLocals::get();
    let info_str = format!("[CPU {}, proc {}]", cpu_locals.processor_id, cpu_locals.current_process.0);
    if let Ok(rust_str) = rust_str {
        println!("{}: {}", info_str, rust_str);
    }
    std::thread::sleep(core::time::Duration::from_millis(10));
}
