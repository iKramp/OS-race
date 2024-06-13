static mut PRINT: Option<fn(&str)> = None;

pub fn set_Print(function: fn(&str)) {
    unsafe { PRINT = Some(function) }
}

pub fn write_text(text: &str) {
    unsafe {
        if let Some(function) = PRINT {
            function(text);
        }
    }
}
