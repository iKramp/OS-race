#[repr(transparent)]
#[derive(Debug)]
pub struct Box<T: 'static> {
    data: &'static T,
}

impl<T> Box<T> {
    pub fn new(data: T) -> Self {
        unsafe {
            let address = crate::HEAP.allocate(crate::mem::size_of::<T>() as u64);
            crate::mem_utils::set_at_virtual_addr(address, data);
            Self {
                data: &*(address.0 as *const T),
            }
        }
    }

    pub fn leak(mut self) {
        unsafe { self.data = &*core::ptr::null() }
    }
}

impl<T> Drop for Box<T> {
    #[inline]
    fn drop(&mut self) {
        if self.data as *const T as u64 != 0 {
            unsafe { crate::HEAP.deallocate(crate::mem_utils::VirtAddr(self.data as *const _ as u64)) }
        }
    }
}
