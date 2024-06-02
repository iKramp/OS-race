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

    pub fn push(&mut self, data: T) {
        unsafe {
            if self.size == self.capacity {
                todo!();
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
            unsafe { Some(&*self.data.add(index)) }
        }
    }
}

impl<T> core::default::Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}
