use core::fmt::Write;

use crate::sync::mutex::{Mutex, MutexGuard};

static mut PRINT: Option<&mut Mutex<dyn Print>> = None;

///# Safety
///printer must be a valid pointer
pub unsafe fn set_print(printer: *mut Mutex<dyn Print>) {
    PRINT = Some(&mut *printer)
}

pub trait Print: Write {
    fn set_bg_color(&mut self, color: (u8, u8, u8));
    fn set_fg_color(&mut self, color: (u8, u8, u8));
    fn reset_color(&mut self);
    fn print(&mut self, args: core::fmt::Arguments) {
        self.write_fmt(args).unwrap();
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! printl {
    ($lock:expr, $($arg:tt)*) => ($crate::print::_print_locked($lock, format_args!($($arg)*)));
}

#[macro_export]
macro_rules! printlnl {
    ($lock:expr, $($arg:tt)*) => ($crate::print_locked!($lock, "{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! printc {
    ($fg:expr, $($arg:tt)*) => ($crate::print::_print_colored($fg, format_args!($($arg)*)));
}

#[macro_export]
macro_rules! printlnc {
    ($fg:expr, $($arg:tt)*) => ($crate::printc!($fg, "{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    unsafe {
        PRINT.as_mut().unwrap().lock().write_fmt(args).unwrap();
    }
}


#[doc(hidden)]
pub fn _print_locked(lock: &mut MutexGuard<dyn Print>, args: core::fmt::Arguments) {
    lock.write_fmt(args).unwrap() ;
}

#[doc(hidden)]
pub fn _print_colored(fg: (u8, u8, u8), args: core::fmt::Arguments) {
    let mut lock = unsafe { PRINT.as_mut().unwrap().lock() };
    lock.set_fg_color(fg);
    _print_locked(&mut lock, args);
    lock.reset_color();
}
