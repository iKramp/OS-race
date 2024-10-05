use core::fmt::{Arguments, Write};

static mut PRINT: Option<&mut dyn Write> = None;

pub fn set_print(printer: &'static mut dyn Write) {
    unsafe { PRINT = Some(printer) }
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

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    unsafe {
        if let Some(print) = &mut PRINT {
            print.write_fmt(args).unwrap()
        } else {
            panic!("trying to println!() without printer");
        }
    }
}
