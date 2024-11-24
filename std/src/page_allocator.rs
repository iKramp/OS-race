use crate::mem_utils::PhysAddr;

pub trait PageAllocator {
    fn allocate(&mut self, physical_address: Option<PhysAddr>) -> crate::mem_utils::VirtAddr;
    fn allocate_set_virtual(&mut self, physical_address: Option<PhysAddr>, virtual_address: crate::mem_utils::VirtAddr);
    fn deallocate(&mut self, addr: crate::mem_utils::VirtAddr);
    fn allocate_contigious(&mut self, num: u64) -> crate::mem_utils::VirtAddr;
}

struct DummyAllocator;
impl PageAllocator for DummyAllocator {
    fn allocate(&mut self, _physical_address: Option<PhysAddr>) -> crate::mem_utils::VirtAddr {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn deallocate(&mut self, _addr: crate::mem_utils::VirtAddr) {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn allocate_contigious(&mut self, _num: u64) -> crate::mem_utils::VirtAddr {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
    fn allocate_set_virtual(&mut self, _physical_address: Option<PhysAddr>, _virtual_address: crate::mem_utils::VirtAddr) {
        panic!("attempted to use the page allocator before setting the static variable to a working allocator");
    }
}

static mut DUMMY_ALLOCATOR: DummyAllocator = DummyAllocator;

#[allow(static_mut_refs)]
pub static mut PAGE_ALLOCATOR: &mut dyn PageAllocator = unsafe { &mut DUMMY_ALLOCATOR };
