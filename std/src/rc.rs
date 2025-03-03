use core::alloc::{GlobalAlloc, Layout};

use crate::mem_utils::VirtAddr;

#[derive(Debug)]
struct RcInner<T: crate::fmt::Debug> {
    data: T,
    count: u64,
}

#[derive(Debug)]
pub struct Rc<T>
where
    T: 'static + crate::fmt::Debug,
{
    inner: &'static mut RcInner<T>,
}

impl<T: crate::fmt::Debug> Rc<T> {
    pub fn new(data: T) -> Self {
        unsafe {
            let layout = Layout::new::<RcInner<T>>();
            let address = crate::HEAP.alloc(layout);
            crate::mem_utils::set_at_virtual_addr(VirtAddr(address as u64), RcInner { data, count: 1 });
            Self {
                inner: &mut *(address as *mut RcInner<T>),
            }
        }
    }

    pub fn get(&self) -> &T {
        &self.inner.data
    }
}

impl<T: crate::fmt::Debug> Drop for Rc<T> {
    #[inline]
    fn drop(&mut self) {
        self.inner.count -= 1;
        if self.inner.count == 0 {
            let layout = Layout::new::<RcInner<T>>();
            unsafe { crate::HEAP.dealloc(self.inner as *mut _ as *mut u8, layout) }
        }
    }
}

impl<T: crate::fmt::Debug> Clone for Rc<T> {
    #[inline]
    fn clone(&self) -> Self {
        unsafe {
            let new_rc = Self {
                inner: crate::mem_utils::get_at_virtual_addr(crate::mem_utils::VirtAddr(self.inner as *const _ as u64)),
            };
            new_rc.inner.count += 1;
            new_rc
        }
    }
}
