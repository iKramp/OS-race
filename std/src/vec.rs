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
            for i in 0..(self.size * crate::mem::size_of::<T>()) {
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

impl<T> AsRef<[T]> for Vec<T> {
    fn as_ref(&self) -> &[T] {
        self.into()
    }
}

impl<T: crate::fmt::Debug> crate::fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_ref().fmt(f)
    }
}
