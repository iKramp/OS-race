use core::fmt::Debug;

use alloc::boxed::Box;

#[derive(Debug)]
struct RcInner<T> {
    data: T,
    count: u64,
}

pub struct Rc<T>
where
    T: 'static,
{
    inner: &'static mut RcInner<T>,
}

impl<T> Rc<T> {
    pub fn new(data: T) -> Self {
        unsafe {
            let address = Box::into_raw(Box::new(RcInner { data, count: 1 })) as usize;
            Self {
                inner: &mut *(address as *mut RcInner<T>),
            }
        }
    }

    pub fn get(&self) -> &T {
        &self.inner.data
    }
}

impl<T> Drop for Rc<T> {
    #[inline]
    fn drop(&mut self) {
        self.inner.count -= 1;
        if self.inner.count == 0 {
            let address = self.inner as *mut RcInner<T>;
            let _ = unsafe { Box::from_raw(address) };
        }
    }
}

impl<T: Debug> Debug for Rc<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rc")
            .field("data", &self.inner.data)
            .field("ref_count", &self.inner.count)
            .finish()
    }
}

impl<T> Clone for Rc<T> {
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
