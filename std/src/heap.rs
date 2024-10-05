use crate::println;

use crate::mem_utils;
use mem_utils::*;

//min allocation is 16 bytes
//16
//32
//64
//128
//256
//512
//1024
//above that we allocate whole pages

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum TypeOfHeap {
    ObjectsInPage,
    ObjectOverPages,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct MultiPageObjectMetadata {
    //should ALWAYS be ObjectOverPages
    type_of_heap: TypeOfHeap,
    pages_allocated: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct HeapPageMetadata {
    //should ALWAYS be ObjectsInPage
    type_of_heap: TypeOfHeap,
    size_order_of_objects: u8,
    number_of_allocations: u8,
    max_allocations: u8,
    ptr_to_first: VirtAddr,
    ptr_to_last: VirtAddr,
}

impl HeapPageMetadata {
    pub fn populate(&mut self, page_addr: VirtAddr) {
        unsafe {
            let size_of_object = u64::pow(2, self.size_order_of_objects as u32);
            let addr_of_first = page_addr + VirtAddr(4096 - size_of_object * self.max_allocations as u64);
            for i in (addr_of_first.0..(page_addr.0 + 4096)).step_by(size_of_object as usize) {
                let empty_block = get_at_virtual_addr::<EmptyBlock>(VirtAddr(i));
                empty_block.ptr_to_prev = VirtAddr(i - size_of_object);
                empty_block.ptr_to_next = VirtAddr(i + size_of_object);
            }
            get_at_virtual_addr::<EmptyBlock>(addr_of_first).ptr_to_prev = page_addr + VirtAddr(4096 - size_of_object);
            get_at_virtual_addr::<EmptyBlock>(page_addr + VirtAddr(4096 - size_of_object)).ptr_to_next = addr_of_first;
            self.ptr_to_first = addr_of_first;
            self.ptr_to_last = page_addr + VirtAddr(4096 - size_of_object);
        }
    }
}

struct EmptyBlock {
    ptr_to_prev: VirtAddr,
    ptr_to_next: VirtAddr,
}

#[derive(Clone, Copy, Debug)]
struct HeapAllocationData {
    size_order_of_objects: u8,
    free_objects: u64,
    ptr_to_first: VirtAddr,
}

impl HeapAllocationData {
    pub const fn new() -> Self {
        Self {
            size_order_of_objects: 0,
            free_objects: 0,
            ptr_to_first: VirtAddr(0),
        }
    }

    pub fn allocate(&mut self) -> VirtAddr {
        unsafe {
            if self.free_objects == 0 {
                let new_page = crate::PAGE_ALLOCATOR.allocate(None);
                let mut metadata = HeapPageMetadata {
                    type_of_heap: TypeOfHeap::ObjectsInPage,
                    size_order_of_objects: self.size_order_of_objects,
                    number_of_allocations: 0,
                    max_allocations: ((4096 - crate::mem::size_of::<HeapPageMetadata>()) as u64
                        / (u64::pow(2, self.size_order_of_objects as u32))) as u8,
                    ptr_to_first: VirtAddr(0),
                    ptr_to_last: VirtAddr(6),
                };
                metadata.populate(new_page);
                set_at_virtual_addr(new_page, metadata);
                self.free_objects = metadata.max_allocations as u64;
                self.ptr_to_first = metadata.ptr_to_first;
            }

            let allocated = self.ptr_to_first;

            let page_metadata = get_at_virtual_addr::<HeapPageMetadata>(VirtAddr(self.ptr_to_first.0 & !0xFFF));
            page_metadata.number_of_allocations += 1;
            self.free_objects -= 1;

            if page_metadata.number_of_allocations < page_metadata.max_allocations {
                let empty_block = get_at_virtual_addr::<EmptyBlock>(allocated);
                let after = empty_block.ptr_to_next;
                page_metadata.ptr_to_first = after;
            }

            if self.free_objects > 0 {
                let empty_block = get_at_virtual_addr::<EmptyBlock>(allocated);
                let before = empty_block.ptr_to_prev;
                let after = empty_block.ptr_to_next;
                let before_block = get_at_virtual_addr::<EmptyBlock>(before);
                let after_block = get_at_virtual_addr::<EmptyBlock>(after);
                self.ptr_to_first = after;
                before_block.ptr_to_next = after;
                after_block.ptr_to_prev = before;
            }
            allocated
        }
    }
    pub fn deallocate(&mut self, addr: VirtAddr) {
        unsafe {
            self.free_objects += 1;
            let metadata = get_at_virtual_addr::<HeapPageMetadata>(VirtAddr(addr.0 & !0b1111_1111_1111));
            metadata.number_of_allocations -= 1;
            let last_block = get_at_virtual_addr::<EmptyBlock>(metadata.ptr_to_last);
            let past_last_block = get_at_virtual_addr::<EmptyBlock>(last_block.ptr_to_next);

            set_at_virtual_addr::<EmptyBlock>(
                addr,
                EmptyBlock {
                    ptr_to_next: last_block.ptr_to_next,
                    ptr_to_prev: metadata.ptr_to_last,
                },
            );
            last_block.ptr_to_next = addr;
            past_last_block.ptr_to_prev = addr;

            //for now i don't deallocate pages lol
        }
    }
}

pub struct Heap {
    allocation_data: [HeapAllocationData; 7],
}

pub static mut HEAP: Heap = Heap::new();

impl Heap {
    pub const fn new() -> Self {
        let mut heap = Self {
            allocation_data: [HeapAllocationData::new(); 7],
        };
        heap.allocation_data[0].size_order_of_objects = 4;
        heap.allocation_data[1].size_order_of_objects = 5;
        heap.allocation_data[2].size_order_of_objects = 6;
        heap.allocation_data[3].size_order_of_objects = 7;
        heap.allocation_data[4].size_order_of_objects = 8;
        heap.allocation_data[5].size_order_of_objects = 9;
        heap.allocation_data[6].size_order_of_objects = 10;
        heap
    }

    pub fn allocate(&mut self, size: u64) -> VirtAddr {
        if size == 0 {
            VirtAddr(self as *const Heap as u64)
        } else if size > 1024 {
            //allocate whole page/pages
            let _n_of_pages = (size + 4095 + crate::mem::size_of::<MultiPageObjectMetadata>() as u64) / 4096;
            todo!("alloc pages");
        } else {
            let size_order = log2_rounded_up(size);
            let index = u64::max(4, size_order) - 4;
            self.allocation_data[index as usize].allocate()
        }
    }

    pub fn deallocate(&mut self, addr: VirtAddr) {
        let page_addr = VirtAddr(addr.0 & !0xFFF);
        unsafe {
            let heap_type = get_at_virtual_addr::<TypeOfHeap>(page_addr);
            if *heap_type == TypeOfHeap::ObjectOverPages {
                let metadata = get_at_virtual_addr::<MultiPageObjectMetadata>(page_addr);
                println!("metadata: {metadata:#x?}");
                todo!("dealloc pages");
            } else {
                let metadata = get_at_virtual_addr::<HeapPageMetadata>(page_addr);
                if metadata.size_order_of_objects < 4 {
                    println!("object has size order: {}", metadata.size_order_of_objects);
                    return;
                }
                let index = metadata.size_order_of_objects - 4;
                self.allocation_data[index as usize].deallocate(addr);
            }
        }
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

///WARNING this function only works for numbers <= 1024
pub fn next_pow_2(mut num: u64) -> u64 {
    let mut first_bit = 0;
    let mut mask = 1_u64 << 9;
    for i in 54..64 {
        if num & mask != 0 {
            first_bit = i;
            break;
        }
        mask >>= 1;
    }
    let mask = u64::MAX >> (first_bit + 1);
    if num & mask != 0 {
        //needs rounding up
        num = 1 << (63 - first_bit + 1);
    }
    num
}

///WARNING this function only works for numbers <= 1024
pub fn log2_rounded_up(num: u64) -> u64 {
    if num == 1 {
        return 0; //special case
    }
    let mut first_bit = 0;
    let mut mask = 1_u64 << 9;
    for i in 54..64 {
        if num & mask != 0 {
            first_bit = i;
            break;
        }
        mask >>= 1;
    }

    let mask = u64::MAX >> (first_bit + 1);
    if num & mask != 0 {
        //needs rounding up
        first_bit -= 1;
    }

    64 - first_bit - 1
}
