#![allow(non_snake_case)]
mod A0_trivial;
mod A1_log_2_rounded_up;
mod A2_vec;
mod memory_utils;

#[cfg(feature = "run_tests")]
static mut FREE_SPACE: [u8; 1032] = [0; 1032];

#[cfg(feature = "run_tests")]
pub(super) fn get_free_space_addr() -> *mut u8 {
    unsafe { (FREE_SPACE.as_mut_ptr() as u64 / 8 * 8) as *mut u8 }
}

#[cfg(feature = "run_tests")]
pub fn test_runner() {
    use std::printlnc;
    use std::println;

    use kernel_test::all_tests;

    let tests = all_tests!();

    printlnc!((0, 255, 255), "Running {} tests", tests.len());
    for (test, name) in tests {
        //TODO add timer when time interrupts work
        println!("testing {name} ... ");
        let passed = test();
        if passed {
            printlnc!((0, 255, 0), "[ok]");
        } else {
            printlnc!((0, 0, 255), "[err]");
        }
    }
}
