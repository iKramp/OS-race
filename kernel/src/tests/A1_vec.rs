use crate::println;
use kernel_test::{kernel_test, kernel_test_mod};
kernel_test_mod!(crate::tests::A1_vec);

#[kernel_test]
fn vec_test_1() -> bool {
    let mut vec1: std::Vec<u8> = std::Vec::new();
    vec1.push(1);
    vec1.push(1);
    vec1.push(2);
    vec1.push(3);
    vec1.push(5);
    vec1.push(8);
    vec1.push(13);
    vec1.push(21);
    vec1.push(34);

    vec1[0] == 1 && vec1[1] == 1 && vec1[2] == 2 && vec1[3] == 3 && vec1[4] == 5 && vec1[5] == 8 && vec1[6] == 13 && vec1[7] == 21 && vec1[8] == 34
}

#[kernel_test]
fn vec_test_2() -> bool {
    let vec1 = std::vec![1, 1, 2, 3, 5, 8, 13, 21, 34];

    vec1[0] == 1 && vec1[1] == 1 && vec1[2] == 2 && vec1[3] == 3 && vec1[4] == 5 && vec1[5] == 8 && vec1[6] == 13 && vec1[7] == 21 && vec1[8] == 34
}

#[kernel_test]
fn vec_test_3() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 3, 5, 8, 13];
    vec1.insert(7, 21);
    vec1.insert(8, 34);

    vec1[0] == 1 && vec1[1] == 1 && vec1[2] == 2 && vec1[3] == 3 && vec1[4] == 5 && vec1[5] == 8 && vec1[6] == 13 && vec1[7] == 21 && vec1[8] == 34
}

#[kernel_test]
fn vec_test_4() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 5, 8, 21, 34];
    vec1.insert(3, 3);
    vec1.insert(6, 13);

    vec1[0] == 1 && vec1[1] == 1 && vec1[2] == 2 && vec1[3] == 3 && vec1[4] == 5 && vec1[5] == 8 && vec1[6] == 13 && vec1[7] == 21 && vec1[8] == 34
}

#[kernel_test]
fn vec_test_5() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 3, 5, 8, 13, 21, 34, 100];
    vec1.remove(9);

    vec1[0] == 1 && vec1[1] == 1 && vec1[2] == 2 && vec1[3] == 3 && vec1[4] == 5 && vec1[5] == 8 && vec1[6] == 13 && vec1[7] == 21 && vec1[8] == 34
}

#[kernel_test]
fn vec_test_6() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 3, 4, 5, 8, 13, 14, 21, 34];
    vec1.remove(4);
    vec1.remove(7);

    vec1[0] == 1 && vec1[1] == 1 && vec1[2] == 2 && vec1[3] == 3 && vec1[4] == 5 && vec1[5] == 8 && vec1[6] == 13 && vec1[7] == 21 && vec1[8] == 34
}
