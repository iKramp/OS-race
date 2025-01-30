#![no_std]

use core::marker::Sized;
use core::option::Option;
use std::Box;

pub trait AmlNew {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)>
    where
        Self: Sized;
}

impl<T: AmlNew> AmlNew for Box<T> {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let ret_data = T::aml_new(data);
        ret_data.map(|(data, skip)| (Box::new(data), skip))
    }
}
