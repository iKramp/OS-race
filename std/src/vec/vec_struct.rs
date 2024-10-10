use core::{
    borrow::Borrow, ops::{Deref, DerefMut}, slice::SliceIndex
};

use crate::{boxed::Box, println};

use super::spec_from_elem::SpecFromElem;

pub struct Vec<T: 'static> {
    size: usize,
    capacity: usize,
    data: *mut T,
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        unsafe {
            Self {
                size: 0,
                capacity: 16,
                data: crate::HEAP.allocate(16 * crate::mem::size_of::<T>() as u64).0 as *mut T,
            }
        }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        unsafe {
            Self {
                size: 0,
                capacity,
                data: crate::HEAP.allocate(capacity as u64 * crate::mem::size_of::<T>() as u64).0 as *mut T,
            }
        }
    }

    fn double_capacity(&mut self) {
        unsafe {
            let new_capacity = self.capacity * 2;
            let new_data = crate::HEAP
                .allocate(new_capacity as u64 * crate::mem::size_of::<T>() as u64)
                .0 as *mut T;
            let size = core::mem::size_of::<T>();
            for i in 0..(self.size * size) {
                crate::mem_utils::set_at_virtual_addr::<u8>(
                    crate::mem_utils::VirtAddr(new_data as u64 + i as u64),
                    *crate::mem_utils::get_at_virtual_addr::<u8>(crate::mem_utils::VirtAddr(self.data as u64 + i as u64)),
                )
            }
            crate::HEAP.deallocate(crate::mem_utils::VirtAddr(self.data as *const _ as u64));
            self.data = new_data;
        }
    }

    pub fn push(&mut self, data: T) {
        unsafe {
            if self.size == self.capacity {
                self.double_capacity();
            }
            *(self.data.add(self.size)) = data;
            self.size += 1;
        }
    }

    pub fn pop(&mut self) -> &T {
        self.size -= 1;
        //look into relocating the vector
        unsafe { &*(self.data.add(self.size)) }
    }

    pub fn at(&self, index: usize) -> Option<&T> {
        if index >= self.size {
            None
        } else {
            Some(self.at_unchecked(index))
        }
    }

    pub fn at_unchecked(&self, index: usize) -> &T {
        unsafe { &*self.data.add(index) }
    }

    pub fn first(&self) -> Option<&T> {
        self.at(0)
    }

    pub fn last(&self) -> Option<&T> {
        self.at(self.size - 1)
    }

    pub fn insert(&mut self, index: usize, data: T) {
        unsafe {
            if self.size == self.capacity {
                self.double_capacity();
            }
            let elem_size = core::mem::size_of::<T>();
            for i in (index..self.size).rev() {
                for j in 0..elem_size {
                    *(self.data as *mut u8).add((i + 1) * elem_size + j) = *(self.data as *mut u8).add(i * elem_size + j);
                }
            }
            *(self.data.add(index)) = data;
            self.size += 1;
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index >= self.size {
            panic!("Index out of bounds");
        }
        unsafe {
            let elem_size = core::mem::size_of::<T>();
            for i in index * elem_size..(self.size - 1) * elem_size {
                *(self.data as *mut u8).add(i) = *(self.data as *mut u8).add(i + elem_size);
            }
            self.size -= 1;
        }
    }
}

pub fn from_elem<T: Clone>(elem: T, n: usize) -> Vec<T> {
    <T as SpecFromElem>::from_elem(elem, n)
}

impl<T> core::default::Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> core::iter::IntoIterator for &'a mut Vec<T> {
    type Item = &'a mut T;

    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        let slice: &'a mut [T] = self.into();
        slice.iter_mut()
    }
}

impl<'a, T> core::iter::IntoIterator for &'a Vec<T> {
    type Item = &'a T;

    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        let slice: &'a [T] = self.into();
        slice.iter()
    }
}

impl<T> core::convert::From<&Vec<T>> for &[T] {
    fn from(value: &Vec<T>) -> Self {
        unsafe { core::slice::from_raw_parts(value.data, value.size) }
    }
}

impl<T> core::convert::From<&mut Vec<T>> for &mut [T] {
    fn from(value: &mut Vec<T>) -> Self {
        unsafe { core::slice::from_raw_parts_mut(value.data, value.size) }
    }
}

impl<T> core::convert::From<Box<[T]>> for Vec<T> {
    fn from(value: Box<[T]>) -> Self {
        let size = value.len();
        Vec {
            size,
            capacity: size,
            data: Box::leak(value) as *mut _ as *mut T,
        }
    
    }
}

impl<T, const N: usize> From<Box<[T; N]>> for Vec<T> {
    fn from(boxed_array: Box<[T; N]>) -> Self {
        Vec {
            size: N,
            capacity: N,
            data: Box::leak(boxed_array) as *mut _ as *mut T,
        }
    }
}

impl<T> AsRef<[T]> for Vec<T> {
    fn as_ref(&self) -> &[T] {
        self.into()
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.into()
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.into()
    }
}

impl<T: crate::fmt::Debug> crate::fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        unsafe {
            crate::HEAP.deallocate(crate::mem_utils::VirtAddr(self.data as *const _ as u64));
        }
    }
}

impl<T, I> crate::ops::Index<I> for Vec<T>
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        core::ops::Index::index(&**self, index)
    }
}

impl<T, I> crate::ops::IndexMut<I> for Vec<T>
where
    I: SliceIndex<[T]>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        core::ops::IndexMut::index_mut(&mut **self, index)
    }
}

impl<T> Eq for Vec<T> where T: Eq {}

impl<T> PartialEq for Vec<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

#[macro_export]
macro_rules! vec {
    () => (
        $crate::Vec::new()
    );
    ($elem:expr; $n:expr) => (
        $crate::vec::from_elem($elem, $n)
    );
    ($($x:expr),+ $(,)?) => (
        $crate::Vec::from(
            // This rustc_box is not required, but it produces a dramatic improvement in compile
            // time when constructing arrays with many elements.
            $crate::boxed::Box::new([$($x),+])
        )
    );
}
