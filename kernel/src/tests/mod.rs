mod A0_trivial;
mod memory_utils;
use crate::println;

#[cfg(feature = "run_tests")]
static mut FREE_SPACE: [u8; 1032] = [0; 1032];

#[cfg(feature = "run_tests")]
pub(super) fn get_free_space_addr() -> *mut u8 {
    unsafe { (FREE_SPACE.as_mut_ptr() as u64 / 8 * 8) as *mut u8 }
}

#[cfg(feature = "run_tests")]
pub fn test_runner() {
    use kernel_test::all_tests;

    use crate::vga::vga_text::set_vga_text_foreground;
    use crate::{print, println};

    let tests = all_tests!();

    set_vga_text_foreground((0, 255, 255));

    println!("Running {} tests", tests.len());
    for (test, name) in tests {
        //TODO add timer when time interrupts work
        set_vga_text_foreground((0, 255, 255));
        println!("testing {name} ... ");
        let passed = test();
        if passed {
            set_vga_text_foreground((0, 255, 0));
            println!("[ok]");
        } else {
            set_vga_text_foreground((0, 0, 255));
            println!("[err]");
        }
    }
    set_vga_text_foreground((255, 255, 255));
}
