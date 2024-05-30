use super::physical_allocator::BUDDY_ALLOCATOR;
use super::utils::*;

struct PageTableEntry(u64);

impl PageTableEntry {
    //creates default entry:
    //present, writeable, not user accessible, not write-through, not cache disabled, not accessed,
    //not dirty, not huge, not global
    pub fn new(address: PhysAddr) -> Self {
        Self((address.0 & 0xF_FFF_FFF_FFF_000) | 0b000000011)
    }

    pub fn address(&self) -> PhysAddr {
        PhysAddr(self.0 & 0xF_FFF_FFF_FFF_000)
    }

    pub fn present(&self) -> bool {
        self.0 & 0b1 != 0
    }

    pub fn set_present(&mut self, present: bool) {
        const OFFSET: u64 = 0;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn writeable(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    pub fn set_writeable(&mut self, present: bool) {
        const OFFSET: u64 = 1;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn user_accessible(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    pub fn set_user_accessible(&mut self, present: bool) {
        const OFFSET: u64 = 2;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn write_through_caching(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    pub fn set_write_through_cahcing(&mut self, present: bool) {
        const OFFSET: u64 = 3;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn disable_cache(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    pub fn set_disable_cahce(&mut self, present: bool) {
        const OFFSET: u64 = 4;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn accessed(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub fn set_accessed(&mut self, present: bool) {
        const OFFSET: u64 = 5;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn dirty(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    pub fn set_dirty(&mut self, present: bool) {
        const OFFSET: u64 = 6;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn huge_page(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn set_huge_page(&mut self, present: bool) {
        const OFFSET: u64 = 7;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn global(&self) -> bool {
        self.0 & (1 << 8) != 0
    }

    pub fn set_global(&mut self, present: bool) {
        const OFFSET: u64 = 8;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn no_execute(&self) -> bool {
        self.0 & (1 << 63) != 0
    }

    pub fn set_no_execute(&mut self, present: bool) {
        const OFFSET: u64 = 63;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }
}

#[repr(align(4096))]
struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = PageTableEntry(0);
        }
    }
}

pub struct PageTree {
    level_4_table: PhysAddr,
}

impl PageTree {
    pub fn new() -> Self {
        let mut level_4_table = PhysAddr(0);
        unsafe {
            core::arch::asm!(
                "mov {}, cr3",
                out(reg) level_4_table.0,
            );
        }
        Self { level_4_table }
    }

    //returns if that address is not yet in use - successful allocation
    pub fn allocate(&self, addr: VirtAddr) -> bool {
        unsafe {
            let mut page_table_address = self.level_4_table;

            for i in 0..3 {
                let page_table = get_at_physical_addr::<PageTable>(page_table_address);
                let entry = &mut page_table.entries[((addr.0 >> (39 - i * 9)) & 0b111_111_111) as usize];
                if !entry.present() {
                    let page_addr = BUDDY_ALLOCATOR.allocate_page();
                    let page = get_at_physical_addr::<PageTable>(page_table_address);
                    page.clear();
                    *entry = PageTableEntry::new(page_addr);
                }
                //means this is the last level page and was already allocated before
                if entry.huge_page() {
                    #[cfg(debug_assertions)]
                    panic!("tried to allocate on a virtual address already mapped to a huge page");
                    return false;
                }

                page_table_address = entry.address();
            }

            //we do level 1 outside of the loop because we have different conditions
            let page_table = get_at_physical_addr::<PageTable>(page_table_address);
            let entry = &mut page_table.entries[(addr.0 >> 12 & 0b111_111_111) as usize];
            if entry.present() {
                #[cfg(debug_assertions)]
                panic!("tried to allocate on a virtual address already mapped to a page");
                return false;
            }
            let page_addr = BUDDY_ALLOCATOR.allocate_page();
            let page = get_at_physical_addr::<PageTable>(page_table_address);
            page.clear();
            *entry = PageTableEntry::new(page_addr);
            true
        }
    }

    fn dealloc_huge_page(&self, entry: &PageTableEntry, level: usize) {
        #[cfg(debug_assertions)]
        assert!(level == 1 || level == 2);

        let physical_address = entry.address();
        let num_to_dealloc = 512 * if level == 1 { 512 } else { 0 };
        for j in 0..num_to_dealloc {
            unsafe {
                BUDDY_ALLOCATOR.deallocate_page(physical_address + PhysAddr(j * 4096));
            }
        }
    }

    pub fn deallocate(&self, addr: VirtAddr) -> bool {
        unsafe {
            let mut frame_address = self.level_4_table;

            for i in 0..4 {
                let page_table = get_at_physical_addr::<PageTable>(frame_address);
                let entry = &mut page_table.entries[((addr.0 >> (39 - i * 9)) & 0b111_111_111) as usize];
                if !entry.present() {
                    #[cfg(debug_assertions)]
                    panic!("tried to deallocate on a virtual address not mapped to a pagee");
                    return false;
                }

                //means this is the last level page
                if entry.huge_page() {
                    self.dealloc_huge_page(entry, i);
                    return true;
                }

                frame_address = entry.address();
            }
            BUDDY_ALLOCATOR.deallocate_page(frame_address);
            true
        }
    }
}
