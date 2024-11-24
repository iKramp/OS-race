use core::ptr::{addr_of_mut, null_mut, DynMetadata};

#[repr(transparent)]
#[derive(Debug)]
pub struct Box<T: ?Sized + 'static> {
    data: &'static mut T,
}

impl<T> Box<T> {
    pub fn new(data: T) -> Self {
        unsafe {
            let address = crate::HEAP.allocate(crate::mem::size_of::<T>() as u64);
            crate::mem_utils::set_at_virtual_addr(address, data);
            Self {
                data: &mut *(address.0 as *mut T),
            }
        }
    }
}

impl <T: ?Sized> Box<T> {
    pub fn leak<'a>(b: Self) -> &'a mut T {
        unsafe { &mut *Box::into_raw(b) }
    }

    pub fn into_raw(b: Self) -> *mut T {
        unsafe { 
            let addr = addr_of_mut!(*(b.data as *const _ as *mut _));
            (&b as *const _ as *mut u64).write(0);
            addr

        }
    }
}


impl<T: ?Sized> Drop for Box<T> {
    #[inline]
    fn drop(&mut self) {
        if self.data as *const T as *const u64 as u64 != 0 {
            unsafe { crate::HEAP.deallocate(crate::mem_utils::VirtAddr(self.data as *const T as *const u64 as u64)) }
        }
    }
}

impl<T> core::ops::Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> core::ops::Deref for Box<[T]> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.data as *const [T]) }
    }
}

impl<T> core::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.data as *mut T) }
    }
}

impl<T> core::ops::DerefMut for Box<[T]> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.data as *mut [T]) }
    }
}
