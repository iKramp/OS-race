use crate::println;
use kernel_test::{kernel_test, kernel_test_mod};
kernel_test_mod!(crate::tests::A0_trivial);

#[kernel_test]
fn trivial_test_1() -> bool {
    0 == 0
}
