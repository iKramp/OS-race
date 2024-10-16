#![allow(incomplete_features)]
#![no_std]
#![feature(ptr_metadata)]
#![feature(specialization)]

mod print;

pub use print::{set_print, test_fn};

pub mod heap;
pub mod mem_utils;
mod page_allocator;
use heap::HEAP;
pub use page_allocator::PageAllocator;
pub use page_allocator::PAGE_ALLOCATOR;

pub use core::alloc;
pub use core::any;
pub use core::arch;
pub use core::array;
pub use core::ascii;
//backtrace
pub mod boxed;
pub use boxed::*;
pub use core::borrow;
pub use core::cell;
pub use core::char;
pub use core::clone;
pub use core::cmp;
//collections
pub use core::convert;
pub use core::default;
pub use core::env;
//error
pub use core::f32;
pub use core::f64;
pub use core::ffi;
pub use core::fmt;
//fs
pub use core::future;
pub use core::hash;
pub use core::hint;
//
//i8   depracation planned
//i16  depracation planned
//i32  depracation planned
//i64  depracation planned
//i128 depracation planned
//io
//isize depracation planned
pub use core::iter;
pub use core::marker;
pub use core::mem;
pub use core::net;
pub use core::num;
pub use core::ops;
pub use core::option;
//os
pub mod panic;
//path
pub use core::pin;
pub use core::prelude;
pub use core::primitive;
//
//process
pub use core::ptr;
pub mod rc;
pub use core::result;
pub use core::slice;
pub use core::str;
//string
pub use core::sync;
pub use core::task;
//thread
pub use core::time;
//u8   depracation planned
//u16  depracation planned
//u32  depracation planned
//u64  depracation planned
//u128 depracation planned
//usize depracation planned
mod vec;
pub use vec::vec_struct::*;
//assert_matches experimental
//async_iter     experimental
//intrinsics     experimental
//simd           experimental
