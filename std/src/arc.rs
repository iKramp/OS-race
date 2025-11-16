use core::fmt::Debug;

use alloc::boxed::Box;

pub struct ArcInner<T: ?Sized> {
    ref_count: core::sync::atomic::AtomicUsize,
    ptr: T,
}

pub struct Arc<T: ?Sized + 'static> {
    inner: &'static mut ArcInner<T>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        let address = Box::into_raw(Box::new(ArcInner {
            ref_count: core::sync::atomic::AtomicUsize::new(1),
            ptr: data,
        })) as usize;
        Self {
            inner: unsafe { &mut *(address as *mut ArcInner<T>) },
        }
    }

    pub fn get(&self) -> &T {
        &self.inner.ptr
    }
}

impl<T: ?Sized + 'static> Clone for Arc<T> {
    fn clone(&self) -> Self {
        self.inner.ref_count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let inner_ptr = self.inner as *const ArcInner<T> as *mut ArcInner<T>;
        let static_inner: &'static mut ArcInner<T> = unsafe { &mut *inner_ptr };

        Self { inner: static_inner }
    }
}

impl<T> Debug for Arc<T>
where
    T: Debug + ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Arc")
            .field("data", &&self.inner.ptr)
            .field(
                "ref_count",
                &self.inner.ref_count.load(core::sync::atomic::Ordering::Relaxed),
            )
            .finish()
    }
}

impl<T: ?Sized> core::ops::Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.ptr
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        unsafe {
            if self.inner.ref_count.fetch_sub(1, core::sync::atomic::Ordering::Relaxed) == 1 {
                let address = self.inner as *mut ArcInner<T>;
                let _ = Box::from_raw(address);
            }
        }
    }
}
