use core::sync::atomic::AtomicUsize;

use alloc::boxed::Box;


struct ArcInner<T> {
    data: T,
    ref_count: AtomicUsize,
}

unsafe impl<T: Send> Send for ArcInner<T> {}
unsafe impl<T: Sync> Sync for ArcInner<T> {}

impl<T> ArcInner<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            ref_count: AtomicUsize::new(1),
        }
    }
}

pub struct Arc<T> {
    inner: *mut ArcInner<T>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        let inner = Box::into_raw(Box::new(ArcInner::new(data)));
        Self {
            inner: unsafe { &mut *inner },
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &(*self.inner).data }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        (unsafe { &mut *self.inner }).ref_count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        Self {
            inner: unsafe { &mut *(self.inner as *const ArcInner<T> as *mut ArcInner<T>) },
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if unsafe { &*self.inner}.ref_count.fetch_sub(1, core::sync::atomic::Ordering::Relaxed) == 1 {
            let _ = unsafe { Box::from_raw(self.inner) };
        }
    }
}
