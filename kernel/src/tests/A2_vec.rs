use crate::println;
use kernel_test::{kernel_test, kernel_test_mod};
kernel_test_mod!(crate::tests::A2_vec);

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

    vec1[0] == 1
        && vec1[1] == 1
        && vec1[2] == 2
        && vec1[3] == 3
        && vec1[4] == 5
        && vec1[5] == 8
        && vec1[6] == 13
        && vec1[7] == 21
        && vec1[8] == 34
}

#[kernel_test]
fn vec_test_2() -> bool {
    let vec1 = std::vec![1, 1, 2, 3, 5, 8, 13, 21, 34];

    vec1[0] == 1
        && vec1[1] == 1
        && vec1[2] == 2
        && vec1[3] == 3
        && vec1[4] == 5
        && vec1[5] == 8
        && vec1[6] == 13
        && vec1[7] == 21
        && vec1[8] == 34
}

#[kernel_test]
fn vec_test_3() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 3, 5, 8, 13];
    vec1.insert(7, 21);
    vec1.insert(8, 34);

    vec1[0] == 1
        && vec1[1] == 1
        && vec1[2] == 2
        && vec1[3] == 3
        && vec1[4] == 5
        && vec1[5] == 8
        && vec1[6] == 13
        && vec1[7] == 21
        && vec1[8] == 34
}

#[kernel_test]
fn vec_test_4() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 5, 8, 21, 34];
    vec1.insert(3, 3);
    vec1.insert(6, 13);

    vec1[0] == 1
        && vec1[1] == 1
        && vec1[2] == 2
        && vec1[3] == 3
        && vec1[4] == 5
        && vec1[5] == 8
        && vec1[6] == 13
        && vec1[7] == 21
        && vec1[8] == 34
}

#[kernel_test]
fn vec_test_5() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 3, 5, 8, 13, 21, 34, 100];
    vec1.remove(9);

    vec1[0] == 1
        && vec1[1] == 1
        && vec1[2] == 2
        && vec1[3] == 3
        && vec1[4] == 5
        && vec1[5] == 8
        && vec1[6] == 13
        && vec1[7] == 21
        && vec1[8] == 34
}

#[kernel_test]
fn vec_test_6() -> bool {
    let mut vec1 = std::vec![1, 1, 2, 3, 4, 5, 8, 13, 14, 21, 34];
    vec1.remove(4);
    vec1.remove(7);

    vec1[0] == 1
        && vec1[1] == 1
        && vec1[2] == 2
        && vec1[3] == 3
        && vec1[4] == 5
        && vec1[5] == 8
        && vec1[6] == 13
        && vec1[7] == 21
        && vec1[8] == 34
}

//push 128 elements test
#[kernel_test]
fn vec_test_7() -> bool {
    let mut vec1: std::Vec<usize> = std::Vec::new();
    for i in 0..128 {
        vec1.push(i);
    }

    for i in 0..128 {
        if vec1[i] != i {
            return false;
        }
    }
    true
}

//pop 128 elements test
#[kernel_test]
fn vec_test_8() -> bool {
    let mut vec1: std::Vec<usize> = std::Vec::new();
    for i in 0..128 {
        vec1.push(i);
    }

    for i in 0..128 {
        if vec1.pop() != Some(127 - i) {
            return false;
        }
    }
    true
}

//remove 128 elements test
#[kernel_test]
fn vec_test_9() -> bool {
    let mut vec1: std::Vec<usize> = std::Vec::new();
    for i in 0..128 {
        vec1.push(i);
    }

    for i in 0..128 {
        if vec1.remove(0) != i {
            return false;
        }
    }
    true
}

//insert 128 elements test
#[kernel_test]
fn vec_test_10() -> bool {
    let mut vec1: std::Vec<usize> = std::Vec::new();
    for i in 0..128 {
        vec1.insert(0, i);
    }

    for i in 0..128 {
        if vec1[i] != 127 - i {
            return false;
        }
    }
    true
}
