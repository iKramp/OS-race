use crate::mem_utils::PhysAddr;

pub trait PageAllocator {
    fn unmap(&mut self, addr: crate::mem_utils::VirtAddr);
    fn allocate(&mut self, physical_address: Option<PhysAddr>) -> crate::mem_utils::VirtAddr;
    fn allocate_set_virtual(&mut self, physical_address: Option<PhysAddr>, virtual_address: crate::mem_utils::VirtAddr);
    fn deallocate(&mut self, addr: crate::mem_utils::VirtAddr);
    fn allocate_contigious(&mut self, num: u64, physical_address: Option<PhysAddr>) -> crate::mem_utils::VirtAddr;
    fn mmap_contigious(&mut self, physical_addresses: &[PhysAddr]) -> crate::mem_utils::VirtAddr;
    fn find_contigious_pages(&mut self, n_pages: usize) -> crate::mem_utils::VirtAddr;
}

struct DummyAllocator;
impl PageAllocator for DummyAllocator {
    fn unmap(&mut self, _addr: crate::mem_utils::VirtAddr) {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn allocate(&mut self, _physical_address: Option<PhysAddr>) -> crate::mem_utils::VirtAddr {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn deallocate(&mut self, _addr: crate::mem_utils::VirtAddr) {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn allocate_contigious(&mut self, _num: u64, _physical_address: Option<PhysAddr>) -> crate::mem_utils::VirtAddr {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn allocate_set_virtual(&mut self, _physical_address: Option<PhysAddr>, _virtual_address: crate::mem_utils::VirtAddr) {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn mmap_contigious(&mut self, _physical_addresses: &[PhysAddr]) -> crate::mem_utils::VirtAddr {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn find_contigious_pages(&mut self, _n_pages: usize) -> crate::mem_utils::VirtAddr {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
}

static mut DUMMY_ALLOCATOR: DummyAllocator = DummyAllocator;

#[allow(static_mut_refs)]
pub static mut PAGE_ALLOCATOR: &mut dyn PageAllocator = unsafe { &mut DUMMY_ALLOCATOR };
