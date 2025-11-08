use core::fmt::{Arguments, Write};

use crate::sync::no_int_spinlock::*;

pub static mut PRINT: Option<&mut NoIntSpinlock<dyn Print>> = None;

///# Safety
///printer must be a valid pointer
pub unsafe fn set_print(printer: *mut NoIntSpinlock<dyn Print>) {
    unsafe { PRINT = Some(&mut *printer) }
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

#[macro_export]
macro_rules! printcl {
    ($fg:expr, $lock:expr, $($arg:tt)*) => ($crate::print::_print_colored_locked($fg, $lock, format_args!($($arg)*)));
}

#[macro_export]
macro_rules! printlncl {
    ($fg:expr, $lock:expr, $($arg:tt)*) => ($crate::printcl!($fg, $lock, "{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    let mut lock = unsafe { PRINT.as_mut().unwrap().lock() };
    _print_locked(&mut lock, args);
}

#[doc(hidden)]
pub fn _print_locked(lock: &mut NoIntSpinlockGuard<dyn Print>, args: core::fmt::Arguments) {
    lock.write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _print_colored(fg: (u8, u8, u8), args: core::fmt::Arguments) {
    let mut lock = unsafe { PRINT.as_mut().unwrap().lock() };
    lock.set_fg_color(fg);
    _print_locked(&mut lock, args);
    lock.reset_color();
}

#[doc(hidden)]
pub fn _print_colored_locked(fg: (u8, u8, u8), lock: &mut NoIntSpinlockGuard<dyn Print>, args: core::fmt::Arguments) {
    lock.set_fg_color(fg);
    _print_locked(lock, args);
    lock.reset_color();
}

#[must_use]
#[inline]
pub fn _format(args: Arguments<'_>) -> crate::String {
    fn format_inner(args: Arguments<'_>) -> crate::String {
        let mut output = crate::String::new();
        output
            .write_fmt(args)
            .expect("a formatting trait implementation returned an error when the underlying stream did not");
        output
    }

    args.as_str()
        .map_or_else(|| format_inner(args), crate::alloc::borrow::ToOwned::to_owned)
}

#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => ($crate::print::_format(core::format_args!($($arg)*)));
}
