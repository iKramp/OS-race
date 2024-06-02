use super::mem_utils::*;
use super::physical_allocator::BUDDY_ALLOCATOR;
use crate::println;

#[derive(Debug, Clone, Copy)]
struct PageTableEntry(u64);

impl PageTableEntry {
    //creates default entry:
    //present, writeable, not user accessible, not write-through, not cache disabled, not accessed,
    //not dirty, not huge, not global
    pub fn new(address: PhysAddr) -> Self {
        let mut entry = Self((address.0 & 0xF_FFF_FFF_FFF_000) | 0b000000011);
        entry.set_num_of_available_pages(512);
        entry
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

    pub fn num_of_available_pages(&self) -> u64 {
        (self.0 >> 52) & 0b1111111111
    }

    fn set_num_of_available_pages(&mut self, num: u64) {
        const MASK: u64 = !(0b11111111111_u64 << 52);
        const NUM_MASK: u64 = !MASK;
        self.0 = (self.0 & MASK) | ((num << 52) & NUM_MASK);
    }

    pub fn decrease_available(&mut self) {
        self.0 -= 1 << 52;
    }

    pub fn increase_available(&mut self) {
        self.0 += 1 << 52;
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
#[derive(Debug)]
struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = PageTableEntry(0);
        }
    }

    pub unsafe fn get_available_entry(&self) -> usize {
        for entry in self.entries.iter().enumerate() {
            if !entry.1.present() || (!entry.1.huge_page() && entry.1.num_of_available_pages() > 0) {
                return entry.0;
            }
        }
        usize::MAX
    }

    pub unsafe fn get_available_entry_level_1(&self) -> usize {
        for entry in self.entries.iter().enumerate() {
            if !entry.1.present() {
                return entry.0;
            }
        }
        usize::MAX
    }

    pub unsafe fn allocate(&mut self) -> VirtAddr {
        let mut address = 0;
        self.allocate_4_to_2(4, &mut address);
        if address & (1 << 47) != 0 {
            address += 0xFFFF << 48; //sign extension
        }
        VirtAddr(address)
    }

    //returns if that page table has less available spaces
    pub unsafe fn allocate_4_to_2(&mut self, level: u64, address: &mut u64) -> bool {
        let index_of_available = self.get_available_entry();

        #[cfg(debug_assertions)]
        if index_of_available == usize::MAX {
            panic!("tried to allocate but could not find available virtual page");
        }

        *address += (index_of_available as u64) << (3 + level * 9);
        let entry = &mut self.entries[index_of_available];

        if !entry.present() {
            let frame_addr = BUDDY_ALLOCATOR.allocate_frame();
            let page_table = get_at_physical_addr::<PageTable>(frame_addr);
            page_table.clear();
            let temp_entry = PageTableEntry::new(frame_addr);
            *entry = temp_entry;
            assert_eq!(temp_entry.0, entry.0);
            assert_eq!(temp_entry.address(), entry.address());
        }

        let lower_page_table = get_at_physical_addr::<PageTable>(entry.address());
        let lower_less_available = if level == 2 {
            lower_page_table.allocate_level_1(address);
            true
        } else {
            lower_page_table.allocate_4_to_2(level - 1, address)
        };
        if lower_less_available {
            entry.decrease_available();
        }
        entry.num_of_available_pages() == 0
    }

    pub unsafe fn allocate_level_1(&mut self, address: &mut u64) {
        let index_of_available = self.get_available_entry_level_1();
        #[cfg(debug_assertions)]
        if index_of_available == usize::MAX {
            panic!("tried to allocate but could not find available virtual page");
        }
        *address += (index_of_available as u64) << 12;
        let entry = &mut self.entries[index_of_available];

        let page_addr = BUDDY_ALLOCATOR.allocate_frame();
        *entry = PageTableEntry::new(page_addr);
    }

    //returns if there was no space but now there is
    pub unsafe fn deallocate(&mut self, address: VirtAddr, level: u64) -> bool {
        let entry = &mut self.entries[(address.0 >> (3 + level * 9) & 0b111_111_111) as usize];
        if level == 1 {
            BUDDY_ALLOCATOR.deallocate_frame(entry.address());
            return true;
        }
        if entry.present() && entry.huge_page() {
            dealloc_huge_page(entry, level);
            return true;
        }
        let lower_level_table = get_at_physical_addr::<PageTable>(entry.address());
        let more_space = lower_level_table.deallocate(address, level - 1);
        if !more_space {
            return false;
        }
        entry.increase_available();
        entry.num_of_available_pages() == 511
    }

    pub fn num_of_available_spaces(&mut self, level: u64) -> u64 {
        let mut sum = 0;
        for entry in &self.entries {
            if !entry.present() {
                sum += 1;
                continue;
            }
            if level == 1 || entry.huge_page() {
                continue;
            }
            unsafe {
                let lower_level_page = get_at_physical_addr::<PageTable>(entry.address());
                let lower_available = lower_level_page.num_of_available_spaces(level - 1);
                if lower_available > 0 {
                    sum += 1;
                }
            }
        }
        sum
    }
}

fn dealloc_huge_page(entry: &PageTableEntry, level: u64) {
    #[cfg(debug_assertions)]
    assert!(level == 2 || level == 3);

    let physical_address = entry.address();
    let num_to_dealloc = 512 * if level == 3 { 512 } else { 1 };
    for j in 0..num_to_dealloc {
        unsafe {
            BUDDY_ALLOCATOR.deallocate_frame(physical_address + PhysAddr(j * 4096));
        }
    }
}

pub struct PageTree {
    pub level_4_table: PhysAddr,
}

impl PageTree {
    pub fn new() -> Self {
        let mut level_4_table = PhysAddr(0);
        unsafe {
            core::arch::asm!(
                "mov {}, cr3",
                out(reg) level_4_table.0,
            );
            let table = get_at_physical_addr::<PageTable>(level_4_table);
            table.num_of_available_spaces(4);
        }
        Self { level_4_table }
    }
}

impl std::PageAllocator for PageTree {
    fn allocate(&mut self) -> std::mem_utils::VirtAddr {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            level_4_table.allocate()
        }
    }

    fn deallocate(&mut self, addr: std::mem_utils::VirtAddr) {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            level_4_table.deallocate(addr, 4);
        }
    }

    fn allocate_contigious(&mut self, num: u64) -> std::mem_utils::VirtAddr {
        todo!()
    }
}
