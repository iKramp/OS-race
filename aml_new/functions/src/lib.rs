#![no_std]

use core::marker::Sized;
use core::option::Option;
use std::Box;

pub trait EnumNew {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)>
    where
        Self: Sized;
}

pub trait StructNew {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)>
    where
        Self: Sized;
}

impl<T: StructNew> StructNew for Box<T> {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let ret_data = T::aml_new(data);
        ret_data.map(|(data, skip)| (Box::new(data), skip))
    }
}

impl<T: EnumNew> EnumNew for Box<T> {
    fn aml_new(data: &[u8]) -> Option<(Self, usize)> {
        let ret_data = T::aml_new(data);
        ret_data.map(|(data, skip)| (Box::new(data), skip))
    }
}
