
use crate::Vec;

// Specialization trait used for Vec::from_elem
#[allow(unused)]//used
pub(super) trait SpecFromElem: Sized {
    fn from_elem(elem: Self, n: usize) -> Vec<Self>;
}

impl<T: Clone> SpecFromElem for T {
    default fn from_elem(elem: Self, n: usize) -> Vec<Self> {
        let mut v = Vec::new_with_capacity(n);
        for _ in 0..n {
            v.push(elem.clone());
        }
        v
    }
}
