use core::ffi::c_char;
use std::println;

pub extern "C" fn console_write(string_ptr: u64, _: u64, _: u64) {
    let str_ptr = unsafe { core::ffi::c_str::CStr::from_ptr(string_ptr as *const c_char) };
    let rust_str = str_ptr.to_str();
    if let Ok(rust_str) = rust_str {
        println!("{}", rust_str);
    }
}
