use core::fmt::{Arguments, Write};

static mut PRINT: Option<&mut dyn Write> = None;

///# Safety 
///printer must be a valid pointer
pub unsafe fn set_print(printer: *mut dyn Write) {
    PRINT = Some(&mut *printer)
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

pub fn test_fn() {
    let a = 0;
    let b = 1;
    let c = a + b;
}
